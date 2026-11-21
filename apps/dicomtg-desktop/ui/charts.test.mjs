import assert from "node:assert/strict";
import test from "node:test";

import {
  donutSegments,
  donutSegmentsFor,
  filterFacetItems,
  filterOverviewItems,
  filterTypeItems,
  mostExpensiveOverviewItem,
  nextChartSelection,
  overviewChartEntries,
  overviewDistribution,
  overviewMetrics,
  orderedManaEntries,
  selectableTypeEntries,
} from "./charts.js";

test("groups the donut tail without losing quantity", () => {
  const entries = Array.from({ length: 10 }, (_, index) => ({
    label: `Catégorie ${index + 1}`,
    quantity: 10 - index,
  }));

  const segments = donutSegments(entries, 5);

  assert.deepEqual(
    segments.map((entry) => entry.label),
    ["Catégorie 1", "Catégorie 2", "Catégorie 3", "Catégorie 4", "Autres"],
  );
  assert.equal(
    segments.reduce((total, entry) => total + entry.quantity, 0),
    entries.reduce((total, entry) => total + entry.quantity, 0),
  );
});

test("ignores empty donut categories", () => {
  assert.deepEqual(
    donutSegments([
      { label: "Visible", quantity: 2 },
      { label: "Vide", quantity: 0 },
    ]),
    [{ label: "Visible", quantity: 2 }],
  );
});

test("keeps every selectable keyword, set and type segment visible", () => {
  const entries = Array.from({ length: 12 }, (_, index) => ({
    label: `Catégorie ${index + 1}`,
    quantity: 12 - index,
  }));

  assert.equal(donutSegmentsFor("keywords", entries).length, 12);
  assert.equal(donutSegmentsFor("sets", entries).length, 12);
  assert.equal(donutSegmentsFor("types", entries).length, 12);
  assert.equal(donutSegmentsFor("rarities", entries).at(-1).label, "Autres");
});

test("orders the mana curve numerically and keeps unknown values last", () => {
  assert.deepEqual(
    orderedManaEntries([
      { label: "10", quantity: 1 },
      { label: "2", quantity: 4 },
      { label: "X", quantity: 2 },
      { label: "0", quantity: 3 },
    ]).map((entry) => entry.label),
    ["0", "2", "10", "X"],
  );
});

test("does not expose Kindred as a standalone selectable type", () => {
  assert.deepEqual(
    selectableTypeEntries([
      { label: "Creature", quantity: 4 },
      { label: "Kindred", quantity: 2 },
      { label: "Enchantment", quantity: 3 },
    ]).map((entry) => entry.label),
    ["Creature", "Enchantment"],
  );
});

test("removes Kindred only from the overview type chart", () => {
  const entries = [
    { label: "Creature", quantity: 4 },
    { label: "Kindred", quantity: 2 },
    { label: "Enchantment", quantity: 3 },
  ];

  assert.deepEqual(
    overviewChartEntries("types", entries).map((entry) => entry.label),
    ["Creature", "Enchantment"],
  );
  assert.equal(overviewChartEntries("colors", entries), entries);
});

test("combines values within a chart with OR and charts with AND", () => {
  const items = [
    { name: "Azorius", colors: ["W", "U"], manaValue: 2 },
    { name: "White", colors: ["W"], manaValue: 3 },
    { name: "Artifact", colors: [], manaValue: 2 },
  ];

  assert.deepEqual(
    filterTypeItems(items, { colors: ["W"], manaValues: ["2"] }).map(
      (item) => item.name,
    ),
    ["Azorius"],
  );
  assert.deepEqual(
    filterTypeItems(items, { colors: ["W", "C"], manaValues: ["2"] }).map(
      (item) => item.name,
    ),
    ["Azorius", "Artifact"],
  );
  assert.deepEqual(filterTypeItems(items, {}).map((item) => item.name), [
    "Azorius",
    "White",
    "Artifact",
  ]);
});

test("uses Ctrl selection to add or remove values", () => {
  assert.deepEqual(nextChartSelection(["W"], "U", true), ["W", "U"]);
  assert.deepEqual(nextChartSelection(["W", "U"], "W", true), ["U"]);
  assert.deepEqual(nextChartSelection(["W", "U"], "B", false), ["B"]);
  assert.deepEqual(nextChartSelection(["W"], "W", false), []);
});

test("cross-filters overview cards with OR inside a chart and AND across charts", () => {
  const items = [
    {
      name: "Serra Angel",
      status: "matched",
      quantity: 2,
      colors: ["W"],
      types: ["Creature"],
      manaValue: 4,
      priceCents: 150,
    },
    {
      name: "Azorius Guildmage",
      status: "matched",
      quantity: 3,
      colors: ["W", "U"],
      types: ["Creature"],
      manaValue: 2,
      priceCents: 100,
    },
    {
      name: "Arcane Signet",
      status: "matched",
      quantity: 1,
      colors: [],
      types: ["Artifact"],
      manaValue: 2,
      priceCents: null,
    },
    {
      name: "Unknown",
      status: "unmatched",
      quantity: 5,
      colors: [],
      types: [],
      manaValue: null,
      priceCents: null,
    },
  ];

  const manaTwo = filterOverviewItems(items, { manaValues: ["2"] });
  assert.deepEqual(manaTwo.map((item) => item.name), [
    "Azorius Guildmage",
    "Arcane Signet",
  ]);
  assert.deepEqual(
    filterOverviewItems(items, {
      types: ["Creature", "Artifact"],
      colors: ["W", "C"],
      manaValues: ["2"],
    }).map((item) => item.name),
    ["Azorius Guildmage", "Arcane Signet"],
  );
  assert.equal(filterOverviewItems(items, { manaValues: ["2"] }, "manaValues").length, 4);

  assert.deepEqual(overviewMetrics(manaTwo), {
    totalQuantity: 4,
    distinctNames: 2,
    matchedPrintings: 2,
    totalValueCents: 300,
  });
  assert.deepEqual(overviewDistribution(manaTwo, "colors"), [
    { label: "W", quantity: 3 },
    { label: "U", quantity: 3 },
    { label: "C", quantity: 1 },
  ]);
  assert.equal(mostExpensiveOverviewItem(manaTwo).name, "Azorius Guildmage");
});

test("filters keyword, set and rarity cards from matched items", () => {
  const items = [
    {
      name: "Angel",
      status: "matched",
      keywords: ["Flying", "Vigilance"],
      setCode: "M21",
      rarity: "rare",
      manaValue: 4,
    },
    {
      name: "Drake",
      status: "matched",
      keywords: ["Flying"],
      setCode: "DOM",
      rarity: "common",
      manaValue: 2,
    },
    {
      name: "Unknown",
      status: "unmatched",
      keywords: [],
      setCode: "M21",
      rarity: null,
      manaValue: null,
    },
  ];

  assert.deepEqual(
    filterFacetItems(items, "keywords", ["Vigilance", "Haste"]).map(
      (item) => item.name,
    ),
    ["Angel"],
  );
  assert.deepEqual(
    filterFacetItems(items, "sets", ["M21", "DOM"]).map((item) => item.name),
    ["Angel", "Drake"],
  );
  assert.deepEqual(
    filterFacetItems(items, "rarities", ["common"]).map((item) => item.name),
    ["Drake"],
  );
  assert.deepEqual(
    filterFacetItems(items, "keywords", []).map((item) => item.name),
    ["Angel", "Drake"],
  );
  assert.deepEqual(
    filterFacetItems(items, "keywords", ["Flying"], ["2"]).map(
      (item) => item.name,
    ),
    ["Drake"],
  );
  assert.deepEqual(
    filterFacetItems(items, "sets", ["M21", "DOM"], ["2", "4"]).map(
      (item) => item.name,
    ),
    ["Angel", "Drake"],
  );
});
