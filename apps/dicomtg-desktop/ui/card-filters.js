const COLOR_BITS = { W: 1, U: 2, B: 4, R: 8, G: 16 };

// The bit positions match the MTGJSON catalog import.
export const CARD_FILTER_FORMATS = ["standard", "pioneer", "modern", "pauper", "legacy", "vintage", "commander"];

function normalize(value) {
  return String(value ?? "").trim().normalize("NFD").replace(/\p{M}/gu, "").toLowerCase();
}

function colorMask(colors) {
  return colors.reduce((mask, color) => mask | (COLOR_BITS[color] ?? 0), 0);
}

function statFilter(value) {
  const term = normalize(value);
  if (!term) return null;
  const comparison = term.match(/^(<=|>=|<|>|=)?\s*(-?\d+(?:[.,]\d+)?)$/);
  if (!comparison) return (stat) => normalize(stat) === term;
  const [, operation, operand] = comparison;
  const target = Number(operand.replace(",", "."));
  return (stat) => {
    if (stat == null || String(stat).trim() === "") return false;
    const number = Number(stat);
    if (!Number.isFinite(number)) return false;
    switch (operation) {
      case "<": return number < target;
      case ">": return number > target;
      case "<=": return number <= target;
      case ">=": return number >= target;
      default: return number === target;
    }
  };
}

const normalizeManaCost = (value) => normalize(value).replace(/[{}\s]/g, "");
const typeFaces = (card) => (card.typeLine ?? "").split("//").map((face) => face.trim().split(/\s+[—–]\s+/, 2));

export function cardSubtypes(card) {
  return card.subtypes ?? typeFaces(card).flatMap(([, subtypes]) => subtypes?.split(/\s+/) ?? []);
}

export function filterCollectionCards(cards, filters = {}) {
  const {
    colors = [], colorMode = "includes", name, text, rarity, type, legendary,
    manaCost, manaValue, power, toughness, loyalty, format, subtype,
  } = filters;
  const selectedColors = colorMask(colors);
  const includeColorless = colors.includes("C");
  const nameTerm = normalize(name);
  const textTerm = normalize(text);
  const typeTerm = normalize(type);
  const subtypeTerm = normalize(subtype);
  const cost = normalizeManaCost(manaCost);
  const formatIndex = CARD_FILTER_FORMATS.indexOf(format);
  const stats = Object.entries({ manaValue, power, toughness, loyalty })
    .map(([key, value]) => [key, statFilter(value)]).filter(([, matches]) => matches);
  return cards.filter((card) => {
    if (nameTerm && !normalize(card.name).includes(nameTerm)) return false;
    if (rarity && card.rarity !== rarity) return false;
    if (colors.length) {
      if (!card.uuid) return false;
      const mask = colorMask(card.colors ?? []);
      if (mask === 0) {
        if (!includeColorless && colorMode !== "atMost") return false;
      } else if (!selectedColors
        || (colorMode === "exact" && mask !== selectedColors)
        || (colorMode === "atMost" && (mask & selectedColors) !== mask)
        || (colorMode === "includes" && (mask & selectedColors) !== selectedColors)) return false;
    }
    if (textTerm && !normalize(card.rulesText).includes(textTerm)) return false;
    if (typeTerm && !card.types?.some((value) => normalize(value) === typeTerm)) return false;
    if (legendary) {
      if (!card.typeLine) return false;
      const isLegendary = typeFaces(card).some(([type]) => /\blegendary\b/i.test(type));
      if (isLegendary !== (legendary === "yes")) return false;
    }
    if (cost && !card.manaCost?.split("//").some((value) => normalizeManaCost(value) === cost)) return false;
    if (stats.some(([key, matches]) => !matches(card[key]))) return false;
    if (formatIndex >= 0 && !(card.legalFormats & (1 << formatIndex))) return false;
    if (subtypeTerm && !cardSubtypes(card).some((value) => normalize(value) === subtypeTerm)) return false;
    return true;
  });
}
