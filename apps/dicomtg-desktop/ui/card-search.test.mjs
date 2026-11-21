import assert from "node:assert/strict";
import test from "node:test";

import {
  cardNameSuggestions,
  findCardsByName,
  groupCardPrintings,
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
