import assert from "node:assert/strict";
import test from "node:test";

import {
  cardNameSuggestions,
  findCardsByName,
  groupCardPrintings,
  groupCardVariants,
} from "./card-search.js";

const cards = [
  { name: "Sol Ring", setCode: "CMM", collectorNumber: "396" },
  { name: "Serra Angel", setCode: "DOM", collectorNumber: "33" },
  { name: "Sol Ring", setCode: "C21", collectorNumber: "263" },
  { name: "Solemn Simulacrum", setCode: "M21", collectorNumber: "239" },
  { name: "Arcane Signet", setCode: "CMM", collectorNumber: "375" },
];

test("suggests at most eight distinct card names with prefix matches first", () => {
  assert.deepEqual(cardNameSuggestions(cards, "sol"), [
    "Sol Ring",
    "Solemn Simulacrum",
  ]);
  assert.equal(cardNameSuggestions(Array.from({ length: 12 }, (_, index) => ({
    name: `Card ${index}`,
  })), "card").length, 8);
});

test("resolves every owned printing from the card name only", () => {
  assert.deepEqual(findCardsByName(cards, "sol ring"), [cards[0], cards[2]]);
  assert.deepEqual(findCardsByName(cards, "serra"), [cards[1]]);
  assert.deepEqual(findCardsByName(cards, "missing"), []);
});

test("merges owned finishes while keeping distinct visual printings", () => {
  const owned = [
    { ...cards[0], uuid: "cmm", finish: "normal", quantity: 2 },
    { ...cards[0], uuid: "cmm", finish: "foil", quantity: 1 },
    { ...cards[2], uuid: "c21", finish: "normal", quantity: 2 },
  ];
  const printings = groupCardPrintings(owned);

  assert.equal(printings.length, 2);
  assert.equal(printings[0].quantity, 2);
  assert.equal(printings[1].quantity, 3);
});

test("groups Wurm printings by their token characteristics under one searchable name", () => {
  const wurm = {
    name: "Wurm", rarity: "token", quantity: 1, power: "5", toughness: "5",
    typeLine: "Token Creature — Wurm", colors: ["G"], colorIdentity: ["G"],
  };
  const amonkhet = { ...wurm, uuid: "akh", setCode: "TAKH", collectorNumber: "24" };
  const foil = { ...amonkhet, finish: "foil", quantity: 2 };
  const baldursGate = {
    ...wurm, uuid: "clb", setCode: "TCLB", collectorNumber: "44",
    typeLine: "Creature — Wurm",
  };
  const mirrodin = {
    ...wurm, uuid: "som", setCode: "TSOM", collectorNumber: "9", quantity: 2,
    power: "3", toughness: "3", typeLine: "Artifact Creature — Wurm",
    rulesText: "Lifelink", keywords: ["Lifelink"], colors: [], colorIdentity: [],
  };
  const owned = [amonkhet, mirrodin, foil, baldursGate];
  assert.deepEqual(cardNameSuggestions(owned, "wurm"), ["Wurm"]);
  const variants = groupCardVariants(findCardsByName(owned, "wurm"));
  assert.deepEqual(variants.map(({ cards }) =>
    groupCardPrintings(cards).map(({ uuid, quantity }) => [uuid, quantity])
  ), [[["akh", 3], ["clb", 1]], [["som", 2]]]);
});

test("keeps distinct rules and unresolved tokens separate without splitting ordinary cards", () => {
  const token = {
    uuid: "token", name: "Elemental", rarity: "token", power: "1", toughness: "1",
    colors: ["U", "R"], typeLine: "Token Creature — Elemental", rulesText: "Flying",
  };
  const repeat = { ...token, uuid: "repeat", colors: ["R", "U"], rulesText: " Flying " };
  const haste = { ...token, uuid: "haste", rulesText: "Haste" };
  const green = { ...token, uuid: "green", colors: ["G"] };
  const unresolved = { name: "Elemental", setCode: "TFOO", collectorNumber: "3" };
  const ordinary = [cards[0], cards[2]];
  assert.deepEqual(
    groupCardVariants([token, repeat, haste, green, unresolved])
      .map(({ cards }) => cards),
    [[token, repeat], [haste], [green], [unresolved]],
  );
  assert.deepEqual(groupCardVariants(ordinary).map(({ cards }) => cards), [ordinary]);
});
