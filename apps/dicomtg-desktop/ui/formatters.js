import { cardLanguage, displayedSetName, magicTerm } from "./game-localization.js";
import { countText, t } from "./interface-language.js";

const numberOptions = {
  count: {},
  size: { maximumFractionDigits: 1 },
  percent: { style: "percent", maximumFractionDigits: 2 },
  currency: { style: "currency", currency: "EUR", minimumFractionDigits: 2, maximumFractionDigits: 2 },
};
let formatterLocale;
const formatters = new Map();

function numberFormatter(kind) {
  if (formatterLocale !== cardLanguage()) {
    formatterLocale = cardLanguage();
    formatters.clear();
  }
  if (!formatters.has(kind)) {
    formatters.set(kind, new Intl.NumberFormat(formatterLocale, numberOptions[kind]));
  }
  return formatters.get(kind);
}

const statusLabels = {
  matched: "Appariée",
  unmatched: "Non appariée",
  ambiguous: "Ambiguë",
};

const finishLabels = {
  normal: "Normale",
  foil: "Foil",
  etched: "Foil gravée",
};

const distributionLabels = {
  colors: {
    W: "Blanc",
    U: "Bleu",
    B: "Noir",
    R: "Rouge",
    G: "Vert",
    C: "Incolore",
  },
  rarities: {
    common: "Commune",
    uncommon: "Peu commune",
    rare: "Rare",
    mythic: "Mythique",
    token: "Jeton",
    special: "Spéciale",
    bonus: "Bonus",
  },
};

export function cardImageUrl(item, size = "normal", face = "front") {
  const imageSize = size === "small" ? "small" : "normal";
  const imageFace = face === "back" ? "back" : "front";

  const scryfallId = item?.scryfallId?.trim().toLowerCase();
  if (/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(scryfallId)) {
    return `https://cards.scryfall.io/${imageSize}/${imageFace}/${scryfallId[0]}/${scryfallId[1]}/${scryfallId}.jpg`;
  }

  if ((!item?.uuid && !item?.catalogOnly) || !item.setCode?.trim() || !item.collectorNumber?.trim()) {
    return null;
  }
  const setCode = encodeURIComponent(item.setCode.trim().toLowerCase());
  const collectorNumber = encodeURIComponent(item.collectorNumber.trim());
  return `https://api.scryfall.com/cards/${setCode}/${collectorNumber}?format=image&version=${imageSize}&face=${imageFace}`;
}

export function distributionLabel(kind, label) {
  if (kind === "sets") return displayedSetName(label);
  if (label === "Autres") return t(label);
  const uiLabel = distributionLabels[kind]?.[label];
  return uiLabel ? t(uiLabel) : magicTerm(kind, label);
}

export function formatCurrency(cents) {
  return numberFormatter("currency").format((cents ?? 0) / 100);
}

export function formatBytes(bytes) {
  if (bytes < 1024) return countText(bytes, "{count} octet", "{count} octets", formatCount(bytes));
  const units = ["Kio", "Mio", "Gio"];
  let value = bytes / 1024;
  let unit = units[0];
  for (const nextUnit of units.slice(1)) {
    if (value < 1024) break;
    value /= 1024;
    unit = nextUnit;
  }
  return `${numberFormatter("size").format(value)} ${t(unit)}`;
}

export function formatPercent(basisPoints) {
  return numberFormatter("percent").format(basisPoints / 10_000);
}

export function formatCount(value) {
  return numberFormatter("count").format(value);
}

export function formatOptionalText(value) {
  return value === null || value === undefined || value === ""
    ? t("Indisponible")
    : String(value);
}

export function formatList(values) {
  return values.length ? values.join(" · ") : t("Indisponible");
}

export function statusLabel(status) {
  return t(statusLabels[status] ?? "Inconnue");
}

export function finishLabel(finish) {
  return t(finishLabels[finish] ?? "Indisponible");
}

export function formatImportSummary(summary) {
  const parts = [
    countText(summary.totalQuantity, "{count} carte importée", "{count} cartes importées", formatCount(summary.totalQuantity)),
    countText(summary.matchedPrintings, "{count} impression appariée", "{count} impressions appariées", formatCount(summary.matchedPrintings)),
  ];
  for (const [count, singular, plural] of [
    [summary.unmatchedRows, "{count} ligne non appariée", "{count} lignes non appariées"],
    [summary.ambiguousRows, "{count} ligne ambiguë", "{count} lignes ambiguës"],
    [summary.invalidRows, "{count} ligne invalide", "{count} lignes invalides"],
    [summary.ignoredRows, "{count} ligne hors des dossiers Magic ignorée", "{count} lignes hors des dossiers Magic ignorées"],
  ]) {
    if (count > 0) parts.push(countText(count, singular, plural, formatCount(count)));
  }
  return parts.join(" · ");
}

export function formatReferenceUpdate(summary) {
  if (summary.status === "running") return t("Une actualisation est déjà en cours.");
  if (summary.status === "current") {
    return t("Données déjà actualisées le {date}.", { date: summary.updateDate });
  }
  return t("{cards} cartes de catalogue · {prices} prix de référence · actualisés le {date}", {
    cards: formatCount(summary.catalogCards), prices: formatCount(summary.referencePrices), date: summary.updateDate,
  });
}

export function formatDiagnosticsSummary(diagnostics) {
  const invalid = diagnostics.invalidRows.length;
  const unresolved = diagnostics.unresolvedRows.length;
  if (invalid === 0 && unresolved === 0) return t("Aucun diagnostic");

  const parts = [];
  if (invalid > 0) {
    parts.push(countText(invalid, "{count} ligne invalide", "{count} lignes invalides", formatCount(invalid)));
  }
  if (unresolved > 0) {
    parts.push(countText(unresolved, "{count} carte à vérifier", "{count} cartes à vérifier", formatCount(unresolved)));
  }
  return parts.join(" · ");
}
