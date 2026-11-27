import { frenchMagicTerms, frenchSetNames } from "./magic-terms-fr.js";

export const CARD_LANGUAGES = Object.freeze({
  fr: "Français",
  en: "English",
  de: "Deutsch",
  es: "Español",
  it: "Italiano",
  pt: "Português",
  ja: "日本語",
});

let language = "fr";
let localizedNames = {};
let localizedSets = {};
let collator = new Intl.Collator(language, { numeric: true, sensitivity: "base" });
const canonicalSearchNames = new WeakMap();
const localizedSearchNames = new WeakMap();
const frenchTerms = Object.fromEntries(Object.entries(frenchMagicTerms).map(([kind, entries]) => [
  kind, new Map(Object.entries(entries).map(([key, value]) => [key.toLowerCase(), value])),
]));

export function setCardLanguage(value) {
  language = Object.hasOwn(CARD_LANGUAGES, value) ? value : "fr";
  collator = new Intl.Collator(language, { numeric: true, sensitivity: "base" });
}

export function cardLanguage() {
  return language;
}

export function setGameLocalizations(names, sets) {
  localizedNames = names ?? {};
  localizedSets = sets ?? {};
}

function aliasesByLanguage(card) {
  return card.localizedNames ?? (Object.hasOwn(localizedNames, card.name) ? localizedNames[card.name] : null);
}

export function displayedCardName(card) {
  return aliasesByLanguage(card)?.[language]?.[0] ?? card.name;
}

export function displayedSetName(code, fallback = code) {
  const names = Object.hasOwn(localizedSets, code) ? localizedSets[code] : null;
  const frenchTitle = language === "fr" && Object.hasOwn(frenchSetNames, code) ? frenchSetNames[code] : null;
  return names?.[language] ?? frenchTitle ?? names?.en ?? fallback;
}

export function magicTerm(kind, value) {
  return (language === "fr" && value ? frenchTerms[kind]?.get(value.toLowerCase()) : null) ?? value;
}

export function compareLocalized(left, right) {
  return collator.compare(left, right);
}

export function normalizeCardSearch(value) {
  const text = String(value ?? "").trim();
  if (/^[\x00-\x7f]*$/.test(text)) return text.toLowerCase();
  return text.normalize("NFD").replace(/(\p{Script=Latin})\p{M}+/gu, "$1").normalize("NFC")
    .toLowerCase().replaceAll("œ", "oe").replaceAll("’", "'");
}

function matchRank(name, query) {
  if (name === query) return 3;
  if (name.startsWith(query)) return 2;
  return name.includes(query) ? 1 : 0;
}

export function cardNameMatchRank(card, normalizedQuery) {
  let canonicalName = canonicalSearchNames.get(card);
  if (!canonicalName || canonicalName.source !== card.name) {
    canonicalName = { source: card.name, normalized: normalizeCardSearch(card.name) };
    canonicalSearchNames.set(card, canonicalName);
  }
  let rank = matchRank(canonicalName.normalized, normalizedQuery);
  if (rank === 3) return rank;

  const aliases = aliasesByLanguage(card);
  if (!aliases) return rank;
  let names = localizedSearchNames.get(aliases);
  if (!names) {
    // Collection printings share one alias object. A refreshed catalog supplies a new one.
    names = [];
    for (const values of Object.values(aliases)) {
      for (const value of values) names.push(normalizeCardSearch(value));
    }
    localizedSearchNames.set(aliases, names);
  }
  for (const name of names) {
    rank = Math.max(rank, matchRank(name, normalizedQuery));
    if (rank === 3) break;
  }
  return rank;
}
