const MAX_SUGGESTIONS = 8;

function normalized(value) {
  return String(value ?? "").trim().toLocaleLowerCase("fr");
}

function matchesFirst(left, right, query) {
  const leftStarts = normalized(left).startsWith(query);
  const rightStarts = normalized(right).startsWith(query);
  if (leftStarts !== rightStarts) return leftStarts ? -1 : 1;
  return left.localeCompare(right, "fr", { sensitivity: "base" });
}

export function cardNameSuggestions(cards, search) {
  const query = normalized(search);
  const names = new Map();
  for (const card of cards) {
    const name = String(card.name ?? "").trim();
    if (name) names.set(normalized(name), name);
  }

  return [...names.values()]
    .filter((name) => normalized(name).includes(query))
    .sort((left, right) => matchesFirst(left, right, query))
    .slice(0, MAX_SUGGESTIONS);
}

export function findCardsByName(cards, search) {
  const query = normalized(search);
  if (!query) return [];
  const match =
    cards.find((card) => normalized(card.name) === query) ??
    cards.find((card) => normalized(card.name).startsWith(query)) ??
    cards.find((card) => normalized(card.name).includes(query));
  if (!match) return [];
  const name = normalized(match.name);
  return cards.filter((card) => normalized(card.name) === name);
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
