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

export function cardNameAliases(card) {
  const aliases = aliasesByLanguage(card);
  return aliases ? [card.name, ...Object.values(aliases).flat()] : [card.name];
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
  return String(value ?? "").trim().normalize("NFD").replace(/(\p{Script=Latin})\p{M}+/gu, "$1").normalize("NFC")
    .toLowerCase().replaceAll("œ", "oe").replaceAll("’", "'");
}

export function cardNameMatchRank(card, normalizedQuery) {
  let rank = 0;
  for (const alias of cardNameAliases(card)) {
    const name = normalizeCardSearch(alias);
    if (name === normalizedQuery) return 3;
    if (name.startsWith(normalizedQuery)) rank = Math.max(rank, 2);
    else if (name.includes(normalizedQuery)) rank = Math.max(rank, 1);
  }
  return rank;
}
