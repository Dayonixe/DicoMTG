import assert from "node:assert/strict";
import test from "node:test";
import { filterCollectionCards } from "./card-filters.js";

const black = { name: "Black", uuid: "b", colors: ["B"], types: ["Creature"], typeLine: "Creature — Goblin" };
const mixed = { name: "Bicolore", uuid: "br", colors: ["B", "R"] };
const artifact = { name: "Artefact", uuid: "a", colors: [] };
const unresolved = { name: "Non appariée", colors: [] };
const cards = [black, mixed, artifact, unresolved];

test("distinguishes colorless, inclusive, exact and at-most color matches", () => {
  assert.deepEqual(filterCollectionCards(cards, { colors: ["C"] }), [artifact]);
  assert.deepEqual(filterCollectionCards(cards, { colors: ["B"] }), [black, mixed]);
  assert.deepEqual(filterCollectionCards(cards, { colors: ["B", "R"] }), [mixed]);
  assert.deepEqual(filterCollectionCards(cards, { colors: ["B"], colorMode: "exact" }), [black]);
  assert.deepEqual(filterCollectionCards(cards, { colors: ["B", "C"], colorMode: "exact" }), [black, artifact]);
  assert.deepEqual(filterCollectionCards(cards, { colors: ["B"], colorMode: "atMost" }), [black, artifact]);
});

test("combines text, type, subtype, supertypes, mana, statistics and format", () => {
  const card = {
    name: "Légende", rulesText: "Target player draws a card.", types: ["Planeswalker"],
    typeLine: "Legendary Planeswalker — Test", subtypes: ["Time Lord"],
    manaCost: "{2}{B}", manaValue: 3, power: "*", toughness: "4", loyalty: "3", legalFormats: 1,
  };
  assert.deepEqual(filterCollectionCards([card, black], {
    name: "Legende", text: "draws a card", type: "Planeswalker", legendary: "yes",
    subtype: "Time Lord", manaCost: "2B", manaValue: ">= 3", power: "*", toughness: "4", loyalty: "<= 3", format: "standard",
  }), [card]);
  assert.deepEqual(filterCollectionCards([card], { format: "commander" }), []);
  assert.deepEqual(filterCollectionCards([card, black, artifact], { legendary: "no" }), [black]);
  assert.deepEqual(filterCollectionCards([black], { subtype: "Goblin" }), [black]);
  assert.deepEqual(filterCollectionCards([card, artifact], { loyalty: "0" }), []);
});
