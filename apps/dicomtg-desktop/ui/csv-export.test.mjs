import assert from "node:assert/strict";
import test from "node:test";
import { cardsTableCsv } from "./csv-export.js";

const card = {
  name: 'Éclat; dit "feu"\r\nsuite',
  setCode: "TST",
  collectorNumber: "001a",
  quantity: 3,
  finish: "normal",
  rarity: "rare",
  colors: ["R", "U"],
  manaValue: 2.5,
  priceCents: 125,
};

test("exports filtered analysis rows in table order as UTF-8 CSV", () => {
  const columns = ["name", "setCode", "quantity", "colors", "manaValue"];
  const items = [card, { ...card, name: "Abrade", quantity: 1, manaValue: 0, colors: [] }];
  assert.equal(cardsTableCsv(items, columns),
    '\uFEFFCarte;Extension;Qté;Couleurs;Mana\r\n' +
    '"Éclat; dit ""feu""\r\nsuite";TST;3;R, U;2,5\r\n' +
    'Abrade;TST;1;;0\r\n');
});

test("keeps printing details and distinguishes an absent price from a real zero", () => {
  const columns = ["name", "setCode", "collectorNumber", "quantity", "finish", "rarity", "colors", "manaValue", "priceCents"];
  const items = [
    { ...card, name: "Zeta" },
    { ...card, name: "Bêta", priceCents: 0 },
    { ...card, name: "Alpha", priceCents: null, manaValue: null },
  ];
  assert.equal(cardsTableCsv(items, columns),
    "\uFEFFCarte;Extension;Numéro;Qté;Finition;Rareté;Couleurs;Mana;Prix (€)\r\n" +
    "Zeta;TST;001a;3;Normale;rare;R, U;2,5;1,25\r\n" +
    "Bêta;TST;001a;3;Normale;rare;R, U;2,5;0,00\r\n" +
    "Alpha;TST;001a;3;Normale;rare;R, U;;\r\n");
});

test("exports sheet-sensitive text as literal text", () => {
  const items = [{ ...card, name: " =1+1" }, { ...card, name: "@SUM(1)" }];
  assert.equal(cardsTableCsv(items, ["name"]), "\uFEFFCarte\r\n' =1+1\r\n'@SUM(1)\r\n");
});
