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
