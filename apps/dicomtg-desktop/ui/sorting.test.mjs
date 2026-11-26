import assert from "node:assert/strict";
import test from "node:test";

import { DEFAULT_CARD_SORT, nextCardSort, sortCards, visibleCardRows } from "./sorting.js";

const cards = [
  { name: "Zeta", quantity: 2, manaValue: null },
  { name: "Alpha", quantity: 1, manaValue: 3 },
  { name: "Beta", quantity: 4, manaValue: 1 },
];

test("bounds the virtual window at the start, middle and end of the collection", () => {
  assert.deepEqual(visibleCardRows(5622, 0, 880), { start: 0, end: 26 });
  assert.deepEqual(visibleCardRows(5622, -4400, 880), { start: 94, end: 126 });
  assert.deepEqual(visibleCardRows(5622, -4400, 440), { start: 94, end: 116 });
  assert.deepEqual(visibleCardRows(5622, -5622 * 44, 880), { start: 5615, end: 5622 });
  assert.deepEqual(visibleCardRows(2, 900, 880), { start: 0, end: 2 });
  assert.deepEqual(visibleCardRows(0, 0, 880), { start: 0, end: 0 });
});

test("sorts cards alphabetically by default without mutating the source", () => {
  assert.deepEqual(sortCards(cards, DEFAULT_CARD_SORT).map((card) => card.name), [
    "Alpha",
    "Beta",
    "Zeta",
  ]);
  assert.deepEqual(cards.map((card) => card.name), ["Zeta", "Alpha", "Beta"]);
});

test("sorts numeric columns in both directions and keeps missing values last", () => {
  assert.deepEqual(
    sortCards(cards, { key: "quantity", direction: "descending" }).map(
      (card) => card.quantity,
    ),
    [4, 2, 1],
  );
  assert.deepEqual(
    sortCards(cards, { key: "manaValue", direction: "descending" }).map(
      (card) => card.name,
    ),
    ["Alpha", "Beta", "Zeta"],
  );
});

test("starts a new column ascending then toggles its direction", () => {
  const quantitySort = nextCardSort(DEFAULT_CARD_SORT, "quantity");
  assert.deepEqual(quantitySort, { key: "quantity", direction: "ascending" });
  assert.deepEqual(nextCardSort(quantitySort, "quantity"), {
    key: "quantity",
    direction: "descending",
  });
});

test("sorts the four Magic rarities in game order and keeps other rarities at the end", () => {
  const items = ["rare", "token", "common", "mythic", null, "uncommon"].map((rarity, index) => ({
    name: String(index), rarity,
  }));
  assert.deepEqual(sortCards(items, { key: "rarity", direction: "ascending" }).map((item) => item.rarity), [
    "common", "uncommon", "rare", "mythic", "token", null,
  ]);
  assert.deepEqual(sortCards(items, { key: "rarity", direction: "descending" }).map((item) => item.rarity), [
    "mythic", "rare", "uncommon", "common", "token", null,
  ]);
});

test("sorts sets by their release dates in both directions, with undated sets last", () => {
  const items = [
    { name: "Zeta", setCode: "BLB" },
    { name: "Alpha", setCode: "ZZZ" },
    { name: "Beta", setCode: "DMU" },
    { name: "Delta", setCode: "TDMU" },
  ];
  const dates = { DMU: "2022-09-09", TDMU: "2022-09-09", BLB: "2024-08-02" };
  const sortedSets = (direction) => sortCards(items, { key: "setCode", direction }, dates)
    .map(({ setCode }) => setCode);
  assert.deepEqual(sortedSets("ascending"), ["DMU", "TDMU", "BLB", "ZZZ"]);
  assert.deepEqual(sortedSets("descending"), ["BLB", "DMU", "TDMU", "ZZZ"]);
});
