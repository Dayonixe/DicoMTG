import { cardNameMatchRank, compareLocalized, displayedCardName, normalizeCardSearch } from "./game-localization.js";

const MAX_SUGGESTIONS = 8;

function normalized(value) {
  return String(value ?? "").trim().toLocaleLowerCase("fr");
}

export function cardNameSuggestions(cards, search) {
  const query = normalizeCardSearch(search);
  const names = new Map();
  for (const card of cards) {
    const key = normalized(card.name);
    if (!key || names.has(key)) continue;
    const rank = query ? cardNameMatchRank(card, query) : 1;
    if (rank) names.set(key, { rank, name: displayedCardName(card), canonical: card.name });
  }

  const labels = new Map();
  for (const { name } of names.values()) labels.set(name, (labels.get(name) ?? 0) + 1);
  return [...names.values()]
    .sort((left, right) => right.rank - left.rank || compareLocalized(left.name, right.name))
    .slice(0, MAX_SUGGESTIONS)
    .map(({ name, canonical }) => labels.get(name) > 1 ? canonical : name);
}

export function findCardsByName(cards, search) {
  const query = normalizeCardSearch(search);
  if (!query) return [];
  let match = cards.find((card) => normalized(card.name) === normalized(search));
  if (!match) {
    let bestRank = 0;
    for (const card of cards) {
      const rank = cardNameMatchRank(card, query);
      if (rank > bestRank) {
        match = card;
        bestRank = rank;
      }
      if (rank === 3) break;
    }
  }
  if (!match) return [];
  const name = normalized(match.name);
  return cards.filter((card) => normalized(card.name) === name);
}

function isToken(card) {
  return normalized(card.rarity) === "token"
    || card.types?.some((type) => normalized(type) === "token")
    || /^token\b/.test(normalized(card.typeLine));
}

function tokenVariantKey(card) {
  const text = (value) => normalized(value).replace(/\s+/g, " ");
  const values = (list) => [...new Set((list ?? []).map(text).filter(Boolean))].sort();
  return JSON.stringify([
    text(card.power),
    text(card.toughness),
    text(card.typeLine).replace(/^token /, "") || values(card.types).join(" "),
    text(card.rulesText),
    values(card.keywords),
    values(card.colors),
    values(card.colorIdentity),
  ]);
}

export function groupCardVariants(cards) {
  if (!cards.some(isToken)) return [{ kind: "card", cards }];

  const groups = new Map();
  for (const card of cards) {
    const kind = isToken(card) ? "token" : card.uuid ? "card" : "unresolved";
    const key = kind === "token" ? tokenVariantKey(card) : kind;
    if (!groups.has(key)) groups.set(key, { kind, cards: [] });
    groups.get(key).cards.push(card);
  }
  return [...groups.values()];
}

const printingCollator = new Intl.Collator("fr", { numeric: true, sensitivity: "base" });

export function groupCardPrintings(cards) {
  const printings = new Map();
  for (const card of cards) {
    const key = card.uuid ?? `${normalized(card.setCode)}\0${card.collectorNumber}`;
    const printing = printings.get(key);
    if (printing) printing.quantity += card.quantity;
    else printings.set(key, { ...card });
  }
  return [...printings.values()].sort(
    (left, right) =>
      printingCollator.compare(left.setCode, right.setCode) ||
      printingCollator.compare(left.collectorNumber, right.collectorNumber),
  );
}
