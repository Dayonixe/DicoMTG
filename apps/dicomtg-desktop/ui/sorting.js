import { finishLabel, statusLabel } from "./formatters.js";

const collator = new Intl.Collator("fr", { numeric: true, sensitivity: "base" });
const rarityRanks = new Map([
  ["common", 0],
  ["uncommon", 1],
  ["rare", 2],
  ["mythic", 3],
]);

const sortValues = {
  name: (item) => item.name,
  collectorNumber: (item) => item.collectorNumber,
  quantity: (item) => item.quantity,
  finish: (item) => finishLabel(item.finish),
  rarity: (item) => rarityRanks.get(item.rarity),
  colors: (item) => (item.colors.length ? item.colors.join(" ") : null),
  manaValue: (item) => item.manaValue,
  priceCents: (item) => item.priceCents,
  status: (item) => statusLabel(item.status),
};

export const DEFAULT_CARD_SORT = Object.freeze({
  key: "name",
  direction: "ascending",
});

export const COLLECTION_ROW_HEIGHT = 44;

export function visibleCardRows(count, bodyTop, viewportHeight) {
  const first = Math.max(0, Math.min(count - 1, Math.floor(-bodyTop / COLLECTION_ROW_HEIGHT)));
  const pastLast = Math.ceil((viewportHeight - bodyTop) / COLLECTION_ROW_HEIGHT);
  return {
    start: Math.max(0, first - 6),
    end: Math.min(count, Math.max(first + 1, pastLast) + 6),
  };
}

export function nextCardSort(current, key) {
  return {
    key,
    direction:
      current.key === key && current.direction === "ascending"
        ? "descending"
        : "ascending",
  };
}

export function sortCards(items, sort = DEFAULT_CARD_SORT, setReleaseDates = {}) {
  const valueFor = sort.key === "setCode"
    ? (item) => Object.hasOwn(setReleaseDates, item.setCode) ? setReleaseDates[item.setCode] : null
    : sortValues[sort.key] ?? sortValues.name;
  const direction = sort.direction === "descending" ? -1 : 1;

  return [...items].sort((left, right) => {
    const leftValue = valueFor(left);
    const rightValue = valueFor(right);
    const leftMissing = leftValue === null || leftValue === undefined || leftValue === "";
    const rightMissing =
      rightValue === null || rightValue === undefined || rightValue === "";

    if (leftMissing !== rightMissing) return leftMissing ? 1 : -1;
    if (!leftMissing) {
      const comparison =
        typeof leftValue === "number" && typeof rightValue === "number"
          ? leftValue - rightValue
          : collator.compare(String(leftValue), String(rightValue));
      if (comparison !== 0) return comparison * direction;
    }
    if (sort.key === "setCode") {
      const byCode = collator.compare(left.setCode ?? "", right.setCode ?? "");
      if (byCode !== 0) return byCode;
    }
    if (sort.key === "rarity" && leftMissing) {
      const byRarity = Number(!left.rarity) - Number(!right.rarity)
        || collator.compare(left.rarity ?? "", right.rarity ?? "");
      if (byRarity !== 0) return byRarity;
    }
    return collator.compare(left.name, right.name);
  });
}
