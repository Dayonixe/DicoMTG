const currencyFormatter = new Intl.NumberFormat("fr-FR", {
  style: "currency",
  currency: "EUR",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

const percentFormatter = new Intl.NumberFormat("fr-FR", {
  style: "percent",
  maximumFractionDigits: 2,
});

const countFormatter = new Intl.NumberFormat("fr-FR");
const sizeFormatter = new Intl.NumberFormat("fr-FR", {
  maximumFractionDigits: 1,
});

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
    special: "Spéciale",
    bonus: "Bonus",
  },
};

export function cardImageUrl(item, size = "normal", face = "front") {
  if (!item?.uuid || !item.setCode?.trim() || !item.collectorNumber?.trim()) {
    return null;
  }
  const imageSize = size === "small" ? "small" : "normal";
  const imageFace = face === "back" ? "back" : "front";

  const scryfallId = item.scryfallId?.trim().toLowerCase();
  if (/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(scryfallId)) {
    return `https://cards.scryfall.io/${imageSize}/${imageFace}/${scryfallId[0]}/${scryfallId[1]}/${scryfallId}.jpg`;
  }

  const setCode = encodeURIComponent(item.setCode.trim().toLowerCase());
  const collectorNumber = encodeURIComponent(item.collectorNumber.trim());
  return `https://api.scryfall.com/cards/${setCode}/${collectorNumber}?format=image&version=${imageSize}&face=${imageFace}`;
}

export function distributionLabel(kind, label) {
  return distributionLabels[kind]?.[label] ?? label;
}

export function formatCurrency(cents) {
  return currencyFormatter.format((cents ?? 0) / 100);
}

export function formatBytes(bytes) {
  if (bytes < 1024) return `${formatCount(bytes)} octet${bytes > 1 ? "s" : ""}`;
  const units = ["Kio", "Mio", "Gio"];
  let value = bytes / 1024;
  let unit = units[0];
  for (const nextUnit of units.slice(1)) {
    if (value < 1024) break;
    value /= 1024;
    unit = nextUnit;
  }
  return `${sizeFormatter.format(value)} ${unit}`;
}

export function formatPercent(basisPoints) {
  return percentFormatter.format(basisPoints / 10_000);
}

export function formatCount(value) {
  return countFormatter.format(value);
}

export function formatOptionalText(value) {
  return value === null || value === undefined || value === ""
    ? "Indisponible"
    : String(value);
}

export function formatList(values) {
  return values.length ? values.join(" · ") : "Indisponible";
}

export function statusLabel(status) {
  return statusLabels[status] ?? "Inconnue";
}

export function finishLabel(finish) {
  return finishLabels[finish] ?? "Indisponible";
}

export function formatImportSummary(summary) {
  const parts = [
    `${formatCount(summary.totalQuantity)} carte${summary.totalQuantity > 1 ? "s" : ""} importée${summary.totalQuantity > 1 ? "s" : ""}`,
    `${formatCount(summary.matchedPrintings)} impression${summary.matchedPrintings > 1 ? "s" : ""} appariée${summary.matchedPrintings > 1 ? "s" : ""}`,
  ];
  if (summary.unmatchedRows > 0) {
    parts.push(
      `${formatCount(summary.unmatchedRows)} ligne${summary.unmatchedRows > 1 ? "s" : ""} non appariée${summary.unmatchedRows > 1 ? "s" : ""}`,
    );
  }
  if (summary.ambiguousRows > 0) {
    parts.push(
      `${formatCount(summary.ambiguousRows)} ligne${summary.ambiguousRows > 1 ? "s" : ""} ambiguë${summary.ambiguousRows > 1 ? "s" : ""}`,
    );
  }
  if (summary.invalidRows > 0) {
    parts.push(
      `${formatCount(summary.invalidRows)} ligne${summary.invalidRows > 1 ? "s" : ""} invalide${summary.invalidRows > 1 ? "s" : ""}`,
    );
  }
  return parts.join(" · ");
}

export function formatReferenceUpdate(summary) {
  if (summary.status === "running") return "Une actualisation est déjà en cours.";
  if (summary.status === "current") {
    return `Données déjà actualisées le ${summary.updateDate}.`;
  }
  return `${formatCount(summary.catalogCards)} carte${summary.catalogCards > 1 ? "s" : ""} de catalogue · ${formatCount(summary.referencePrices)} prix de référence · actualisés le ${summary.updateDate}`;
}

export function formatDiagnosticsSummary(diagnostics) {
  const invalid = diagnostics.invalidRows.length;
  const unresolved = diagnostics.unresolvedRows.length;
  if (invalid === 0 && unresolved === 0) return "Aucun diagnostic";

  const parts = [];
  if (invalid > 0) {
    parts.push(`${formatCount(invalid)} ligne${invalid > 1 ? "s" : ""} invalide${invalid > 1 ? "s" : ""}`);
  }
  if (unresolved > 0) {
    parts.push(`${formatCount(unresolved)} carte${unresolved > 1 ? "s" : ""} à vérifier`);
  }
  return parts.join(" · ");
}
