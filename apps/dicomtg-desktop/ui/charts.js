export const CHART_PALETTE = [
  "#d9ad63",
  "#69a7c8",
  "#a989c2",
  "#df7969",
  "#72ad75",
  "#b9a77e",
  "#75b6a4",
  "#c885a4",
];

const semanticColors = {
  colors: {
    W: "#ead9af",
    U: "#62a9d1",
    B: "#9a80ad",
    R: "#df6f5f",
    G: "#68aa70",
    C: "#9ca3ad",
  },
  rarities: {
    common: "#aeb4be",
    uncommon: "#6eacd2",
    rare: "#d9ad63",
    mythic: "#dc784e",
    special: "#a989c2",
    bonus: "#75b6a4",
  },
};

export function chartColor(kind, label, index) {
  return semanticColors[kind]?.[label] ?? CHART_PALETTE[index % CHART_PALETTE.length];
}

export function donutSegments(entries, limit = 8) {
  const positive = entries.filter((entry) => entry.quantity > 0);
  if (positive.length <= limit) return positive.map((entry) => ({ ...entry }));

  // shortcut: crowded donuts group their tail; the adjacent ranking keeps every category.
  const visible = positive.slice(0, limit - 1).map((entry) => ({ ...entry }));
  visible.push({
    label: "Autres",
    quantity: positive
      .slice(limit - 1)
      .reduce((total, entry) => total + entry.quantity, 0),
  });
  return visible;
}

export function donutSegmentsFor(kind, entries) {
  return donutSegments(
    entries,
    kind === "keywords" || kind === "sets" || kind === "types" ? Infinity : 8,
  );
}

export function orderedManaEntries(entries) {
  return [...entries].sort((left, right) => {
    const leftValue = Number(left.label);
    const rightValue = Number(right.label);
    const leftIsNumber = Number.isFinite(leftValue);
    const rightIsNumber = Number.isFinite(rightValue);
    if (leftIsNumber && rightIsNumber) return leftValue - rightValue;
    if (leftIsNumber) return -1;
    if (rightIsNumber) return 1;
    return left.label.localeCompare(right.label, "fr");
  });
}

export function selectableTypeEntries(entries) {
  return entries.filter((entry) => entry.label.toLowerCase() !== "kindred");
}

export function overviewChartEntries(kind, entries) {
  return kind === "types" ? selectableTypeEntries(entries) : entries;
}

export function nextChartSelection(selected, label, additive) {
  if (additive) {
    return selected.includes(label)
      ? selected.filter((value) => value !== label)
      : [...selected, label];
  }
  return selected.length === 1 && selected[0] === label ? [] : [label];
}

export function filterTypeItems(
  items,
  { colors = [], manaValues = [] } = {},
) {
  return items.filter((item) => {
    const matchesColor =
      colors.length === 0 ||
      colors.some((color) =>
        color === "C" ? item.colors.length === 0 : item.colors.includes(color),
      );
    const matchesMana =
      manaValues.length === 0 || manaValues.includes(String(item.manaValue));
    return matchesColor && matchesMana;
  });
}

export function filterOverviewItems(items, filters = {}, ignoredKind = null) {
  const active = ["colors", "types", "manaValues"].some(
    (kind) => kind !== ignoredKind && (filters[kind]?.length ?? 0) > 0,
  );
  if (!active) return [...items];

  return items.filter((item) => {
    if (item.status !== "matched") return false;
    const matchesColors =
      ignoredKind === "colors" ||
      !filters.colors?.length ||
      filters.colors.some((color) =>
        color === "C" ? item.colors.length === 0 : item.colors.includes(color),
      );
    const matchesTypes =
      ignoredKind === "types" ||
      !filters.types?.length ||
      filters.types.some((type) => item.types.includes(type));
    const matchesMana =
      ignoredKind === "manaValues" ||
      !filters.manaValues?.length ||
      filters.manaValues.includes(String(item.manaValue));
    return matchesColors && matchesTypes && matchesMana;
  });
}

export function overviewMetrics(items) {
  const matched = items.filter((item) => item.status === "matched");
  return {
    totalQuantity: items.reduce((total, item) => total + item.quantity, 0),
    distinctNames: new Set(
      items.map((item) => item.name.trim().toLocaleLowerCase("fr")),
    ).size,
    matchedPrintings: matched.length,
    totalValueCents: matched.reduce(
      (total, item) => total + (item.priceCents ?? 0) * item.quantity,
      0,
    ),
  };
}

export function mostExpensiveOverviewItem(items) {
  return items.reduce(
    (highest, item) =>
      item.status === "matched" &&
      item.priceCents !== null &&
      (!highest || item.priceCents > highest.priceCents)
        ? item
        : highest,
    null,
  );
}

export function overviewDistribution(items, kind) {
  const quantities = new Map();
  for (const item of items) {
    if (item.status !== "matched") continue;
    let labels;
    if (kind === "colors") labels = item.colors.length ? item.colors : ["C"];
    else if (kind === "types") {
      labels = item.types.filter((type) => type.toLowerCase() !== "kindred");
    } else if (kind === "manaValues" && item.manaValue !== null) {
      labels = [String(item.manaValue)];
    } else labels = [];
    for (const label of labels) {
      quantities.set(label, (quantities.get(label) ?? 0) + item.quantity);
    }
  }

  const colorOrder = ["W", "U", "B", "R", "G", "C"];
  return [...quantities.entries()]
    .map(([label, quantity]) => ({ label, quantity }))
    .sort(
      (left, right) =>
        right.quantity - left.quantity ||
        (kind === "colors"
          ? colorOrder.indexOf(left.label) - colorOrder.indexOf(right.label)
          : left.label.localeCompare(right.label, "fr", {
              numeric: true,
              sensitivity: "base",
            })),
    );
}

export function filterFacetItems(items, kind, selected, manaValues = []) {
  return items.filter((item) => {
    if (item.status !== "matched") return false;
    const values =
      kind === "keywords"
        ? item.keywords
        : kind === "sets"
          ? [item.setCode]
          : item.rarity
            ? [item.rarity]
            : [];
    const matchesFacet =
      values.length > 0 &&
      (selected.length === 0 || selected.some((value) => values.includes(value)));
    const matchesMana =
      manaValues.length === 0 || manaValues.includes(String(item.manaValue));
    return matchesFacet && matchesMana;
  });
}
