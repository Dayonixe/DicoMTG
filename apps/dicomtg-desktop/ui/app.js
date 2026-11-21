import {
  cardImageUrl,
  distributionLabel,
  formatBytes,
  formatCount,
  formatCurrency,
  formatDiagnosticsSummary,
  finishLabel,
  formatImportSummary,
  formatList,
  formatOptionalText,
  formatPercent,
  formatReferenceUpdate,
  statusLabel,
} from "./formatters.js";
import {
  chartColor,
  donutSegmentsFor,
  filterFacetItems,
  filterOverviewItems,
  filterTypeItems,
  mostExpensiveOverviewItem,
  nextChartSelection,
  overviewDistribution,
  overviewMetrics,
  orderedManaEntries,
  selectableTypeEntries,
} from "./charts.js";
import {
  cardNameSuggestions,
  findCardsByName,
  groupCardPrintings,
} from "./card-search.js";
import { PAGE_IDS, pageFromHash } from "./navigation.js";
import { DEFAULT_CARD_SORT, nextCardSort, sortCards } from "./sorting.js";
import { manaSymbolParts } from "./mana-symbols.js";
import { calculateBasicLands, DECK_COLOR_CODES } from "./deck-builder.js";

const form = document.querySelector("#filters");
const collectionSearch = document.querySelector("#search");
const cardNameSuggestionsList = document.querySelector("#card-name-suggestions");
const cardDetailSearchForm = document.querySelector("#card-detail-search-form");
const cardDetailSearch = document.querySelector("#card-detail-search");
const cardDetailSuggestionsList = document.querySelector(
  "#card-detail-suggestions",
);
const cardDetailSearchError = document.querySelector("#card-detail-search-error");
const submitButton = document.querySelector("#apply-filters");
const importForm = document.querySelector("#collection-import");
const importTitle = document.querySelector("#import-title");
const importButton = document.querySelector("#import-collection");
const collectionNameInput = document.querySelector("#collection-name");
const fileInput = document.querySelector("#collection-file");
const collectionUrlInput = document.querySelector("#collection-url");
const collectionDropZone = document.querySelector("#collection-drop-zone");
const collectionFileStatus = document.querySelector("#collection-file-status");
const cancelCollectionUpdateButton = document.querySelector(
  "#cancel-collection-update",
);
const importError = document.querySelector("#collection-import-error");
const importResult = document.querySelector("#import-result");
const importSummary = document.querySelector("#import-summary");
const importIssues = document.querySelector("#import-issues");
const importIssueList = document.querySelector("#import-issue-list");
const managedCollectionsList = document.querySelector("#managed-collections");
const managedCollectionsStatus = document.querySelector(
  "#managed-collections-status",
);
const managedCollectionsError = document.querySelector(
  "#managed-collections-error",
);
const referenceButton = document.querySelector("#update-reference-data");
const referenceError = document.querySelector("#reference-error");
const referenceResult = document.querySelector("#reference-result");
const imageCacheUsage = document.querySelector("#image-cache-usage");
const imageCacheProgress = document.querySelector("#image-cache-progress");
const imageCacheError = document.querySelector("#image-cache-error");
const clearImageCacheButton = document.querySelector("#clear-image-cache");
const catalogStatus = document.querySelector("#catalog-status");
const priceStatus = document.querySelector("#price-status");
const diagnosticsSummary = document.querySelector("#diagnostics-summary");
const invalidCount = document.querySelector("#invalid-count");
const unresolvedCount = document.querySelector("#unresolved-count");
const invalidDiagnostics = document.querySelector("#invalid-diagnostics");
const unresolvedDiagnostics = document.querySelector("#unresolved-diagnostics");
const statusRegion = document.querySelector("#load-status");
const errorRegion = document.querySelector("#load-error");
const rows = document.querySelector("#collection-rows");
const cardDetailEmpty = document.querySelector("#card-detail-empty");
const cardDetailContent = document.querySelector("#card-detail-content");
const cardDetailTitle = document.querySelector("#card-detail-title");
const cardDetailFacts = document.querySelector("#card-detail-facts");
const cardDetailRules = document.querySelector("#card-detail-rules");
const cardDetailPrintings = document.querySelector("#card-detail-printings");
const mostExpensiveEmpty = document.querySelector("#most-expensive-empty");
const mostExpensiveCard = document.querySelector("#most-expensive-card");
const mostExpensiveImage = document.querySelector("#most-expensive-image");
const mostExpensiveImageState = document.querySelector("#most-expensive-image-state");
const mostExpensivePrice = document.querySelector("#most-expensive-price");
const mostExpensiveContext = document.querySelector("#most-expensive-context");
const mostExpensiveDetail = document.querySelector("#most-expensive-detail");
const overviewFilterSummary = document.querySelector("#overview-filter-summary");
const resetOverviewChartFilters = document.querySelector(
  "#reset-overview-chart-filters",
);
const cardPreview = document.querySelector("#card-preview");
const cardPreviewImage = document.querySelector("#card-preview-image");
const cardPreviewImageState = document.querySelector("#card-preview-image-state");
const cardPreviewName = document.querySelector("#card-preview-name");
const cardPreviewMeta = document.querySelector("#card-preview-meta");
const typeSelect = document.querySelector("#type-filter");
const typeAnalysisError = document.querySelector("#type-analysis-error");
const typeAnalysisStatus = document.querySelector("#type-analysis-status");
const typeRows = document.querySelector("#type-analysis-rows");
const typeResultSummary = document.querySelector("#type-result-summary");
const resetTypeChartFilters = document.querySelector(
  "#reset-type-chart-filters",
);
const deckNumberInputs = new Map(
  [...document.querySelectorAll("[data-deck-number]")].map((input) => [
    input.dataset.deckNumber,
    input,
  ]),
);
const deckRangeInputs = new Map(
  [...document.querySelectorAll("[data-deck-range]")].map((input) => [
    input.dataset.deckRange,
    input,
  ]),
);
const deckLandStack = document.querySelector("#deck-land-stack");
const deckLandLegend = document.querySelector("#deck-land-legend");
const deckLandMessage = document.querySelector("#deck-land-message");
const distributionLists = {
  colors: document.querySelectorAll('[data-distribution="colors"]'),
  types: document.querySelectorAll('[data-distribution="types"]'),
  keywords: document.querySelectorAll('[data-distribution="keywords"]'),
  sets: document.querySelectorAll('[data-distribution="sets"]'),
  rarities: document.querySelectorAll('[data-distribution="rarities"]'),
  manaValues: document.querySelectorAll('[data-distribution="manaValues"]'),
};
const chartTargets = {
  colors: document.querySelectorAll('[data-chart="colors"]'),
  types: document.querySelectorAll('[data-chart="types"]'),
  keywords: document.querySelectorAll('[data-chart="keywords"]'),
  sets: document.querySelectorAll('[data-chart="sets"]'),
  rarities: document.querySelectorAll('[data-chart="rarities"]'),
  manaValues: document.querySelectorAll('[data-chart="manaValues"]'),
};
const facetKinds = ["keywords", "sets", "rarities"];
const facetAnalyses = Object.fromEntries(
  facetKinds.map((kind) => [
    kind,
    {
      target: chartTargets[kind][0],
      manaTarget: document.querySelector(`[data-facet-mana-chart="${kind}"]`),
      metrics: {
        quantity: document.querySelector(`#${kind}-metric-quantity`),
        distinctNames: document.querySelector(`#${kind}-metric-distinct-names`),
        value: document.querySelector(`#${kind}-metric-value`),
      },
      rows: document.querySelector(`#${kind}-analysis-rows`),
      summary: document.querySelector(`#${kind}-result-summary`),
      reset: document.querySelector(`#reset-${kind}-chart-filters`),
      entries: null,
      manaEntries: [],
      filters: { facet: [], manaValues: [] },
    },
  ]),
);
const typeChartTargets = {
  colors: document.querySelector('[data-type-chart="colors"]'),
  manaValues: document.querySelector('[data-type-chart="manaValues"]'),
};
const chartNames = {
  colors: "Couleurs",
  types: "Types",
  keywords: "Mots-clés",
  sets: "Extensions",
  rarities: "Raretés",
  manaValues: "Valeurs de mana",
};
const deckColorNames = {
  W: "Blanc",
  U: "Bleu",
  B: "Noir",
  R: "Rouge",
  G: "Vert",
};
const SVG_NAMESPACE = "http://www.w3.org/2000/svg";
const cardImageRequestIds = new WeakMap();
const cardImageObjectUrls = new WeakMap();
const PREVIEW_HOVER_DELAY_MS = 350;
const PREVIEW_FOCUS_DELAY_MS = 150;
let cardPreviewTimer;
let cardPreviewTarget;
let cardDetailRequestId = 0;
let searchableCards = [];
let overviewDashboard;
const overviewChartFilters = { colors: [], types: [], manaValues: [] };
let typeAnalysis;
const typeChartFilters = { colors: [], manaValues: [] };
const tableSortStates = new WeakMap();
const analysisContexts = new WeakMap();
let collectionItems = [];
let managedCollections = [];
let collectionUpdateTarget;
let selectedImportFile;
const pageElements = new Map(
  PAGE_IDS.map((page) => [page, document.querySelector(`[data-page="${page}"]`)]),
);
const navigationLinks = document.querySelectorAll("[data-nav-page]");

const invoke = window.__TAURI__?.core?.invoke;

function showPage(page) {
  hideCardPreview();
  for (const [kind, analysis] of Object.entries(facetAnalyses)) {
    if (kind !== page) analysis.rows.replaceChildren();
  }
  for (const [pageId, element] of pageElements) {
    element.hidden = pageId !== page;
  }
  for (const link of navigationLinks) {
    if (link.dataset.navPage === page) link.setAttribute("aria-current", "page");
    else link.removeAttribute("aria-current");
  }
  document.title = `${pageElements.get(page).dataset.pageTitle} · DicoMTG`;
}

function routeFromLocation() {
  const page = pageFromHash(window.location.hash);
  if (window.location.hash !== `#${page}`) {
    window.history.replaceState(null, "", `#${page}`);
  }
  showPage(page);
  const facetAnalysis = facetAnalyses[page];
  if (facetAnalysis?.entries) renderFacetAnalysis(page, facetAnalysis.entries);
  if (page === "data") loadImageCacheStats();
}

function navigateToPage(page) {
  if (window.location.hash === `#${page}`) showPage(page);
  else window.location.hash = page;
}

function currentQuery() {
  const data = new FormData(form);
  const rarity = data.get("rarity");
  return {
    search: String(data.get("search") ?? ""),
    colors: data.getAll("color").map(String),
    rarities: rarity ? [String(rarity)] : [],
  };
}

function hasActiveQuery(query) {
  return Boolean(query.search.trim() || query.colors.length || query.rarities.length);
}

function replaceSuggestions(list, values) {
  list.replaceChildren(
    ...values.map((value) => {
      const option = document.createElement("option");
      option.value = value;
      return option;
    }),
  );
}

function refreshNameSuggestions() {
  replaceSuggestions(
    cardNameSuggestionsList,
    cardNameSuggestions(searchableCards, collectionSearch.value),
  );
}

function refreshDetailSuggestions() {
  replaceSuggestions(
    cardDetailSuggestionsList,
    cardNameSuggestions(searchableCards, cardDetailSearch.value),
  );
}

function updateSearchableCards(cards) {
  searchableCards = cards;
  refreshNameSuggestions();
  refreshDetailSuggestions();
}

function setLoading(loading) {
  submitButton.disabled = loading;
  submitButton.textContent = loading ? "Chargement…" : "Appliquer";
}

function setImportLoading(loading) {
  importButton.disabled = loading;
  fileInput.disabled = loading;
  collectionUrlInput.disabled = loading;
  collectionNameInput.disabled = loading || Boolean(collectionUpdateTarget);
  cancelCollectionUpdateButton.disabled = loading;
  importButton.textContent = loading
    ? "Import en cours…"
    : collectionUpdateTarget
      ? "Mettre à jour"
      : "Ajouter";
}

function showError(message) {
  errorRegion.textContent = message;
  errorRegion.hidden = false;
  statusRegion.textContent = "Échec du chargement";
}

function clearError() {
  errorRegion.textContent = "";
  errorRegion.hidden = true;
}

function showImportError(message) {
  importError.textContent = message;
  importError.hidden = false;
  fileInput.setAttribute("aria-invalid", "true");
  collectionUrlInput.setAttribute("aria-invalid", "true");
  collectionNameInput.setAttribute("aria-invalid", "true");
  statusRegion.textContent = "Échec de l’import";
}

function clearImportError() {
  importError.textContent = "";
  importError.hidden = true;
  fileInput.removeAttribute("aria-invalid");
  collectionUrlInput.removeAttribute("aria-invalid");
  collectionNameInput.removeAttribute("aria-invalid");
}

function setReferenceLoading(loading) {
  referenceButton.disabled = loading;
  referenceButton.textContent = loading ? "Actualisation en cours…" : "Actualiser";
}

function showReferenceError(message) {
  referenceError.textContent = message;
  referenceError.hidden = false;
  statusRegion.textContent = "Échec de l’actualisation";
}

function clearReferenceError() {
  referenceError.textContent = "";
  referenceError.hidden = true;
}

function renderImageCacheStats(stats) {
  imageCacheUsage.textContent = `${formatBytes(stats.totalBytes)} sur ${formatBytes(stats.capacityBytes)} · ${formatCount(stats.imageCount)} image${stats.imageCount > 1 ? "s" : ""}`;
  imageCacheProgress.max = stats.capacityBytes || 1;
  imageCacheProgress.value = Math.min(stats.totalBytes, stats.capacityBytes);
  clearImageCacheButton.disabled = stats.imageCount === 0;
}

async function loadImageCacheStats() {
  imageCacheError.hidden = true;
  imageCacheUsage.textContent = "Calcul de l’occupation…";
  clearImageCacheButton.disabled = true;
  try {
    if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
    renderImageCacheStats(await invoke("load_image_cache_stats"));
  } catch (error) {
    imageCacheError.textContent = String(error);
    imageCacheError.hidden = false;
    imageCacheUsage.textContent = "Occupation indisponible";
  }
}

function renderOverviewMetrics(metrics) {
  document.querySelector("#metric-quantity").textContent = formatCount(
    metrics.totalQuantity,
  );
  document.querySelector("#metric-distinct-names").textContent = formatCount(
    metrics.distinctNames,
  );
  document.querySelector("#metric-distinct-printings").textContent = formatCount(
    metrics.matchedPrintings,
  );
  document.querySelector("#metric-value").textContent = formatCurrency(
    metrics.totalValueCents,
  );
}

function renderMetrics(metrics, itemCount) {
  renderOverviewMetrics(metrics);
  document.querySelector("#metric-coverage").textContent = formatPercent(
    metrics.priceCoverageBasisPoints,
  );
  document.querySelector("#metric-unpriced").textContent = formatCount(
    metrics.unpricedPrintings,
  );
  catalogStatus.textContent = metrics.catalogCards
    ? `${formatCount(metrics.catalogCards)} cartes`
    : "Non installé";
  priceStatus.textContent = metrics.referencePrices
    ? `${formatCount(metrics.referencePrices)} prix`
    : "Non installé";
  document.querySelector("#result-summary").textContent = `${formatCount(itemCount)} résultat${itemCount > 1 ? "s" : ""}`;
}

function appendCell(row, value, className) {
  const cell = document.createElement("td");
  if (value instanceof Node) cell.append(value);
  else cell.textContent = value;
  if (className) cell.className = className;
  row.append(cell);
}

function appendDetailFact(label, value) {
  const fact = document.createElement("div");
  const term = document.createElement("dt");
  const description = document.createElement("dd");
  term.textContent = label;
  if (value instanceof Node) description.append(value);
  else description.textContent = value;
  fact.append(term, description);
  cardDetailFacts.append(fact);
}

function createManaSymbolText(value, fallback = "Indisponible") {
  const content = String(value ?? "").trim() || fallback;
  const container = document.createElement("span");
  container.className = "mana-symbol-text";
  for (const part of manaSymbolParts(content)) {
    if (typeof part === "string") {
      container.append(document.createTextNode(part));
      continue;
    }
    const image = document.createElement("img");
    image.className = "mana-symbol";
    image.src = part.url;
    image.alt = part.label;
    image.title = part.label;
    image.decoding = "async";
    image.referrerPolicy = "no-referrer";
    image.addEventListener(
      "error",
      () => image.replaceWith(document.createTextNode(part.symbol)),
      { once: true },
    );
    container.append(image);
  }
  return container;
}

function createColorSymbols(colors, fallback = "—") {
  return createManaSymbolText(
    colors.map((color) => `{${color}}`).join(""),
    fallback,
  );
}

function setManaSymbolText(target, value, fallback) {
  target.replaceChildren(createManaSymbolText(value, fallback));
}

function clearCardImage(image, state) {
  cardImageRequestIds.set(image, (cardImageRequestIds.get(image) ?? 0) + 1);
  const objectUrl = cardImageObjectUrls.get(image);
  if (objectUrl) URL.revokeObjectURL(objectUrl);
  cardImageObjectUrls.delete(image);
  image.hidden = true;
  image.removeAttribute("src");
  image.alt = "";
  state.hidden = false;
}

async function showCardImage(item, image, state, size = "normal", face = "front") {
  clearCardImage(image, state);
  const requestId = cardImageRequestIds.get(image);
  const isCurrent = () => cardImageRequestIds.get(image) === requestId;
  let imageUrl = cardImageUrl(item, size, face);

  if (!imageUrl) {
    state.textContent = "Illustration indisponible pour cette carte non appariée.";
    return;
  }

  state.textContent = "Chargement de l’illustration…";
  image.alt = `${face === "back" ? "Verso" : "Recto"} de la carte Magic : ${item.name}`;
  if (invoke) {
    try {
      const bytes = await invoke("load_card_image_bytes", {
        scryfallId: item.scryfallId ?? null,
        setCode: item.setCode,
        collectorNumber: item.collectorNumber,
        size,
        face,
      });
      if (!isCurrent()) return;
      imageUrl = URL.createObjectURL(new Blob([bytes], { type: "image/jpeg" }));
      cardImageObjectUrls.set(image, imageUrl);
    } catch {
      if (!isCurrent()) return;
      imageUrl = cardImageUrl(item, size, face);
    }
  }

  image.onload = () => {
    if (!isCurrent()) return;
    image.hidden = false;
    state.hidden = true;
  };
  image.onerror = () => {
    if (!isCurrent()) return;
    image.hidden = true;
    state.textContent = "Illustration indisponible hors ligne ou pour cette impression.";
    state.hidden = false;
  };
  image.src = imageUrl;
}

function positionCardPreview(target) {
  const gap = 12;
  const edge = 12;
  const targetBox = target.getBoundingClientRect();
  const previewBox = cardPreview.getBoundingClientRect();
  let left = targetBox.right + gap;
  if (left + previewBox.width > window.innerWidth - edge) {
    left = targetBox.left - previewBox.width - gap;
  }
  const top = Math.min(
    Math.max(targetBox.top - gap, edge),
    window.innerHeight - previewBox.height - edge,
  );
  cardPreview.style.left = `${Math.max(left, edge)}px`;
  cardPreview.style.top = `${Math.max(top, edge)}px`;
}

function openCardPreview(item, target) {
  cardPreviewTarget = target;
  cardPreviewName.textContent = item.name;
  cardPreviewMeta.textContent = `${item.setCode} · n° ${item.collectorNumber}`;
  cardPreview.hidden = false;
  target.setAttribute("aria-describedby", "card-preview");
  positionCardPreview(target);
  showCardImage(item, cardPreviewImage, cardPreviewImageState, "small");
}

function scheduleCardPreview(item, target, delay) {
  window.clearTimeout(cardPreviewTimer);
  cardPreviewTimer = window.setTimeout(() => {
    cardPreviewTimer = undefined;
    openCardPreview(item, target);
  }, delay);
}

function hideCardPreview() {
  window.clearTimeout(cardPreviewTimer);
  cardPreviewTimer = undefined;
  if (cardPreviewTarget) cardPreviewTarget.removeAttribute("aria-describedby");
  cardPreviewTarget = undefined;
  if (!cardPreview.hidden) {
    clearCardImage(cardPreviewImage, cardPreviewImageState);
  }
  cardPreview.hidden = true;
}

function attachCardPreview(target, item) {
  target.addEventListener("mouseenter", () => {
    scheduleCardPreview(item, target, PREVIEW_HOVER_DELAY_MS);
  });
  target.addEventListener("mouseleave", () => {
    if (document.activeElement !== target) hideCardPreview();
  });
  target.addEventListener("focus", () => {
    scheduleCardPreview(item, target, PREVIEW_FOCUS_DELAY_MS);
  });
  target.addEventListener("blur", hideCardPreview);
}

function clearCardDetailPrintings() {
  for (const image of cardDetailPrintings.querySelectorAll("img")) {
    clearCardImage(image, image.parentElement.querySelector("p"));
  }
  cardDetailPrintings.replaceChildren();
}

function renderCardPrinting(item) {
  const printing = document.createElement("article");
  printing.className = "card-printing";
  const visual = document.createElement("figure");
  visual.className = "card-printing-visual";
  const frame = document.createElement("div");
  frame.className = "card-image-frame";
  const image = document.createElement("img");
  image.className = "card-detail-image";
  image.decoding = "async";
  image.referrerPolicy = "no-referrer";
  image.hidden = true;
  const imageState = document.createElement("p");
  imageState.setAttribute("aria-live", "polite");
  imageState.textContent = "Illustration en attente.";
  frame.append(image, imageState);

  if (item.hasBackFace) {
    let face = "front";
    const flip = document.createElement("button");
    flip.className = "card-detail-flip";
    flip.type = "button";
    flip.innerHTML = '<span aria-hidden="true">↻</span>';
    const updateFlip = () => {
      const label = face === "front" ? "Afficher le verso" : "Afficher le recto";
      flip.setAttribute("aria-label", label);
      flip.title = label;
    };
    updateFlip();
    flip.addEventListener("click", () => {
      face = face === "front" ? "back" : "front";
      updateFlip();
      showCardImage(item, image, imageState, "normal", face);
    });
    frame.append(flip);
  }

  const caption = document.createElement("figcaption");
  const setName = document.createElement("strong");
  setName.className = "card-printing-set";
  setName.textContent = item.setCode;
  const number = document.createElement("span");
  number.className = "card-printing-number";
  number.textContent = `n° ${item.collectorNumber}`;
  caption.append(setName, number);
  visual.append(frame, caption);

  const prices = document.createElement("dl");
  prices.className = "card-printing-prices";
  for (const [label, value] of [
    ["Non-foil", item.normalPriceCents],
    ["Foil", item.foilPriceCents],
  ]) {
    const price = document.createElement("div");
    const term = document.createElement("dt");
    const amount = document.createElement("dd");
    term.textContent = label;
    amount.textContent = formatCurrency(value);
    price.append(term, amount);
    prices.append(price);
  }

  printing.append(visual, prices);
  cardDetailPrintings.append(printing);
  showCardImage(item, image, imageState);
  return { item, setName };
}

async function localizeCardDetail(printings, requestId) {
  if (!invoke) return;
  let hasFrenchText = false;
  for (const { item, setName } of printings) {
    try {
      const localization = await invoke("load_card_localization", {
        setCode: item.setCode,
        collectorNumber: item.collectorNumber,
      });
      if (requestId !== cardDetailRequestId) return;
      setName.textContent = localization.setName;
      if (!hasFrenchText && localization.frenchText) {
        setManaSymbolText(cardDetailRules, localization.frenchText);
        hasFrenchText = true;
      }
    } catch {
      // The local catalog remains a complete offline fallback.
    }
  }
}

function showCardDetail(item) {
  hideCardPreview();
  const matchingCards = findCardsByName(searchableCards, item.name);
  const cards = matchingCards.length > 0 ? matchingCards : [item];
  const representative = cards.find((card) => card.uuid) ?? cards[0];
  const requestId = ++cardDetailRequestId;
  cardDetailSearch.value = representative.name;
  cardDetailSearchError.hidden = true;
  cardDetailTitle.textContent = representative.name;
  cardDetailFacts.replaceChildren();
  clearCardDetailPrintings();

  const mergedValues = (key) => [
    ...new Set(cards.flatMap((card) => card[key] ?? []).filter(Boolean)),
  ];
  const rarities = [
    ...new Set(cards.map((card) => card.rarity).filter(Boolean)),
  ].map((rarity) => distributionLabel("rarities", rarity));
  const facts = [
    [
      "Quantité possédée",
      formatCount(cards.reduce((total, card) => total + card.quantity, 0)),
    ],
    ["Rareté", formatList(rarities)],
    ["Valeur de mana", formatOptionalText(representative.manaValue)],
    ["Coût de mana", createManaSymbolText(representative.manaCost)],
    ["Couleurs", createColorSymbols(mergedValues("colors"), "Indisponible")],
    [
      "Identité couleur",
      createColorSymbols(mergedValues("colorIdentity"), "Indisponible"),
    ],
    ["Types", formatList(mergedValues("types"))],
    ["Mots-clés", formatList(mergedValues("keywords"))],
    ["Force", formatOptionalText(representative.power)],
    ["Endurance", formatOptionalText(representative.toughness)],
  ];
  for (const [label, value] of facts) appendDetailFact(label, value);
  setManaSymbolText(
    cardDetailRules,
    formatOptionalText(representative.rulesText),
  );

  const printings = groupCardPrintings(cards).map(renderCardPrinting);
  void localizeCardDetail(printings, requestId);
  cardDetailEmpty.hidden = true;
  cardDetailContent.hidden = false;
  navigateToPage("card");
  window.requestAnimationFrame(() => cardDetailTitle.focus());
}

function renderMostExpensiveCard(item) {
  if (!item) {
    clearCardImage(mostExpensiveImage, mostExpensiveImageState);
    mostExpensiveCard.hidden = true;
    mostExpensiveEmpty.hidden = false;
    return;
  }

  mostExpensivePrice.textContent = formatCurrency(item.priceCents);
  const quantity = `${formatCount(item.quantity)} exemplaire${item.quantity > 1 ? "s" : ""}`;
  const total = formatCurrency(item.priceCents * item.quantity);
  mostExpensiveContext.textContent = `${quantity} possédé${item.quantity > 1 ? "s" : ""} · valeur totale ${total}`;
  mostExpensiveDetail.onclick = () => showCardDetail(item);
  mostExpensiveEmpty.hidden = true;
  mostExpensiveCard.hidden = false;
  showCardImage(item, mostExpensiveImage, mostExpensiveImageState);
}

function sortStateFor(rowsTarget) {
  return tableSortStates.get(rowsTarget) ?? DEFAULT_CARD_SORT;
}

function updateSortHeaders(table, state) {
  for (const header of table.tHead.rows[0].cells) {
    header.removeAttribute("aria-sort");
    if (header.dataset.sortKey === state.key) {
      header.setAttribute("aria-sort", state.direction);
    }
  }
}

function rerenderSortedTable(rowsTarget) {
  if (rowsTarget === rows) {
    renderItems(collectionItems);
    return;
  }
  const context = analysisContexts.get(rowsTarget);
  if (context) {
    renderAnalysisItems(
      context.items,
      rowsTarget,
      context.summaryTarget,
      context.emptyMessage,
    );
  }
}

function initializeSortableTables() {
  for (const table of document.querySelectorAll("table[data-sort-keys]")) {
    const keys = table.dataset.sortKeys.split(",");
    const headers = [...table.tHead.rows[0].cells];
    const rowsTarget = table.tBodies[0];
    if (!rowsTarget || keys.length !== headers.length) {
      throw new Error("Configuration de tri de tableau invalide.");
    }

    tableSortStates.set(rowsTarget, DEFAULT_CARD_SORT);
    headers.forEach((header, index) => {
      header.dataset.sortKey = keys[index];
      const label = header.textContent.trim();
      const button = document.createElement("button");
      button.type = "button";
      button.className = "sort-button";
      button.textContent = label;
      button.setAttribute("aria-label", `Trier par ${label}`);
      button.addEventListener("click", () => {
        const state = nextCardSort(sortStateFor(rowsTarget), keys[index]);
        tableSortStates.set(rowsTarget, state);
        updateSortHeaders(table, state);
        rerenderSortedTable(rowsTarget);
      });
      header.replaceChildren(button);
    });
    updateSortHeaders(table, DEFAULT_CARD_SORT);
  }
}

function renderItems(items) {
  hideCardPreview();
  collectionItems = items;
  rows.replaceChildren();
  if (items.length === 0) {
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.colSpan = 10;
    cell.className = "empty-state";
    cell.textContent = "Aucune carte ne correspond aux filtres actuels.";
    row.append(cell);
    rows.append(row);
    return;
  }

  for (const item of sortCards(items, sortStateFor(rows))) {
    const row = document.createElement("tr");
    const nameCell = document.createElement("td");
    const detailButton = document.createElement("button");
    detailButton.type = "button";
    detailButton.className = "card-detail-trigger";
    detailButton.textContent = item.name;
    detailButton.setAttribute("aria-label", `Consulter la fiche de ${item.name}`);
    detailButton.addEventListener("click", () => showCardDetail(item));
    attachCardPreview(detailButton, item);
    nameCell.append(detailButton);
    row.append(nameCell);
    appendCell(row, item.setCode);
    appendCell(row, item.collectorNumber);
    appendCell(row, formatCount(item.quantity), "numeric");
    appendCell(row, finishLabel(item.finish));
    appendCell(row, item.rarity ?? "Indisponible");
    appendCell(row, createColorSymbols(item.colors));
    appendCell(
      row,
      item.manaValue === null ? "Indisponible" : String(item.manaValue),
      "numeric",
    );
    appendCell(row, formatCurrency(item.priceCents), "price");

    const statusCell = document.createElement("td");
    const badge = document.createElement("span");
    badge.className = `status-badge status-badge--${item.status}`;
    badge.textContent = statusLabel(item.status);
    statusCell.append(badge);
    row.append(statusCell);
    rows.append(row);
  }
}

function renderDistribution(kind, entries) {
  for (const list of distributionLists[kind]) {
    list.replaceChildren();
    if (entries.length === 0) {
      const empty = document.createElement("li");
      empty.className = "distribution-empty";
      empty.textContent = "Aucune donnée disponible.";
      list.append(empty);
      continue;
    }

    const maximum = entries[0].quantity;
    for (const entry of entries) {
      const item = document.createElement("li");
      const label = document.createElement("span");
      const quantity = document.createElement("strong");
      const bar = document.createElement("span");
      const fill = document.createElement("span");
      if (kind === "colors") label.append(createColorSymbols([entry.label]));
      else label.textContent = distributionLabel(kind, entry.label);
      quantity.textContent = formatCount(entry.quantity);
      bar.className = "distribution-bar";
      bar.setAttribute("aria-hidden", "true");
      fill.style.width = `${(entry.quantity / maximum) * 100}%`;
      bar.append(fill);
      item.append(label, quantity, bar);
      list.append(item);
    }
  }
}

function renderChartMessage(target, message) {
  const text = document.createElement("p");
  text.className = "chart-empty";
  text.textContent = message;
  target.classList.remove("chart-target--tooltip");
  target.replaceChildren(text);
}

function renderChartEmpty(target) {
  renderChartMessage(target, "Aucune donnée disponible.");
}

function renderDonutChart(
  kind,
  entries,
  target,
  { selectedLabels = [], onSelect = null, showTooltip = false } = {},
) {
  const segments = donutSegmentsFor(kind, entries);
  if (segments.length === 0) {
    renderChartEmpty(target);
    return;
  }

  const total = segments.reduce((sum, entry) => sum + entry.quantity, 0);
  const colors = segments.map((entry, index) => chartColor(kind, entry.label, index));

  const layout = document.createElement("div");
  layout.className = "donut-layout";
  const plot = document.createElement("div");
  plot.className = "donut-plot";
  const svg = document.createElementNS(SVG_NAMESPACE, "svg");
  svg.classList.add("donut-svg");
  svg.setAttribute("viewBox", "0 0 100 100");
  svg.setAttribute("role", onSelect || showTooltip ? "group" : "img");
  svg.setAttribute(
    "aria-label",
    `${chartNames[kind]} : ${segments
      .map((entry) => `${distributionLabel(kind, entry.label)} ${formatCount(entry.quantity)}`)
      .join(", ")}`,
  );

  const tooltip = document.createElement("span");
  tooltip.className = "chart-tooltip";
  tooltip.setAttribute("role", "tooltip");
  tooltip.hidden = true;
  let cursor = 0;
  for (const [index, entry] of segments.entries()) {
    const percentage = Math.round((entry.quantity * 10_000) / total);
    const segment = document.createElementNS(SVG_NAMESPACE, "circle");
    segment.classList.add("donut-segment");
    segment.setAttribute("cx", "50");
    segment.setAttribute("cy", "50");
    segment.setAttribute("r", "38");
    segment.setAttribute("pathLength", "100");
    segment.setAttribute("fill", "none");
    segment.setAttribute("stroke", colors[index]);
    segment.setAttribute("stroke-width", "24");
    segment.setAttribute(
      "stroke-dasharray",
      `${(entry.quantity / total) * 100} ${100 - (entry.quantity / total) * 100}`,
    );
    segment.setAttribute("stroke-dashoffset", String(-cursor));
    segment.dataset.chartValue = entry.label;
    cursor += (entry.quantity / total) * 100;

    const tooltipText = `${distributionLabel(kind, entry.label)} · ${formatCount(
      entry.quantity,
    )} · ${formatPercent(percentage)}`;
    if (showTooltip) {
      segment.classList.add("donut-segment--hoverable");
      const show = () => {
        tooltip.textContent = tooltipText;
        tooltip.hidden = false;
      };
      const hide = () => {
        tooltip.hidden = true;
      };
      segment.addEventListener("mouseenter", show);
      segment.addEventListener("mouseleave", hide);
      segment.addEventListener("focus", show);
      segment.addEventListener("blur", hide);
      if (!onSelect) {
        segment.setAttribute("tabindex", "0");
        segment.setAttribute("role", "img");
        segment.setAttribute("aria-label", tooltipText);
      }
    }

    if (onSelect) {
      const selected = selectedLabels.includes(entry.label);
      segment.classList.add("donut-segment--interactive");
      segment.classList.toggle("is-selected", selected);
      segment.classList.toggle("is-muted", selectedLabels.length > 0 && !selected);
      segment.setAttribute("tabindex", "0");
      segment.setAttribute("role", "button");
      segment.setAttribute("aria-pressed", String(selected));
      segment.setAttribute(
        "aria-label",
        `${tooltipText}. ${selected ? "Retirer le filtre" : "Filtrer les cartes"}. Ctrl+clic pour une sélection multiple`,
      );
      segment.addEventListener("click", (event) => {
        onSelect(entry.label, event.ctrlKey || event.metaKey);
      });
      segment.addEventListener("keydown", (event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        onSelect(entry.label, event.ctrlKey || event.metaKey);
      });
    }
    svg.append(segment);
  }

  const center = document.createElement("span");
  center.className = "donut-center";
  const totalValue = document.createElement("strong");
  totalValue.textContent = formatCount(total);
  const totalLabel = document.createElement("span");
  totalLabel.textContent = "contributions";
  center.append(totalValue, totalLabel);
  plot.append(svg, center);
  if (showTooltip) plot.append(tooltip);

  const legend = document.createElement("ul");
  legend.className = "chart-legend";
  for (const [index, entry] of segments.entries()) {
    const item = document.createElement("li");
    const swatch = document.createElement("span");
    const label = document.createElement("span");
    const value = document.createElement("strong");
    swatch.className = "chart-swatch";
    swatch.style.background = colors[index];
    swatch.setAttribute("aria-hidden", "true");
    if (kind === "colors") label.append(createColorSymbols([entry.label]));
    else label.textContent = distributionLabel(kind, entry.label);
    value.textContent = `${formatCount(entry.quantity)} · ${formatPercent(
      Math.round((entry.quantity * 10_000) / total),
    )}`;
    const selected = selectedLabels.includes(entry.label);
    item.classList.toggle("is-selected", selected);
    if (onSelect) {
      item.classList.add("chart-legend-entry--interactive");
      item.dataset.chartValue = entry.label;
      item.setAttribute("tabindex", "0");
      item.setAttribute("role", "button");
      item.setAttribute("aria-pressed", String(selected));
      item.setAttribute(
        "aria-label",
        `${distributionLabel(kind, entry.label)}, ${value.textContent}. ${
          selected ? "Retirer le filtre" : "Filtrer les cartes"
        }. Ctrl+clic pour une sélection multiple`,
      );
      item.addEventListener("click", (event) => {
        onSelect(entry.label, event.ctrlKey || event.metaKey);
      });
      item.addEventListener("keydown", (event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        onSelect(entry.label, event.ctrlKey || event.metaKey);
      });
    }
    item.append(swatch, label, value);
    legend.append(item);
  }

  layout.append(plot, legend);
  target.replaceChildren(layout);
}

function renderManaChart(
  entries,
  target,
  { selectedLabels = [], onSelect = null, showTooltip = false } = {},
) {
  const ordered = orderedManaEntries(entries);
  if (ordered.length === 0) {
    renderChartEmpty(target);
    return;
  }

  const maximum = Math.max(...ordered.map((entry) => entry.quantity));
  const scroll = document.createElement("div");
  scroll.className = "column-chart-scroll";
  const plot = document.createElement("div");
  plot.className = "column-chart";
  plot.setAttribute("role", onSelect || showTooltip ? "group" : "img");
  plot.setAttribute(
    "aria-label",
    `Courbe de mana : ${ordered
      .map((entry) => `valeur ${entry.label}, ${formatCount(entry.quantity)}`)
      .join(", ")}`,
  );

  const tooltip = document.createElement("span");
  tooltip.className = "chart-tooltip chart-tooltip--column";
  tooltip.setAttribute("role", "tooltip");
  tooltip.hidden = true;
  for (const entry of ordered) {
    const column = document.createElement(onSelect ? "button" : "div");
    column.className = "chart-column";
    column.dataset.chartValue = entry.label;
    const tooltipText = `Mana ${entry.label} · ${formatCount(entry.quantity)}`;
    if (showTooltip) {
      column.classList.add("chart-column--hoverable");
      const show = () => {
        tooltip.textContent = tooltipText;
        tooltip.hidden = false;
      };
      const hide = () => {
        tooltip.hidden = true;
      };
      column.addEventListener("mouseenter", show);
      column.addEventListener("mouseleave", hide);
      column.addEventListener("focus", show);
      column.addEventListener("blur", hide);
      if (!onSelect) {
        column.setAttribute("tabindex", "0");
        column.setAttribute("role", "img");
        column.setAttribute("aria-label", tooltipText);
      }
    }
    if (onSelect) {
      const selected = selectedLabels.includes(entry.label);
      column.type = "button";
      column.setAttribute("aria-pressed", String(selected));
      column.setAttribute(
        "aria-label",
        `Valeur de mana ${entry.label}, ${formatCount(entry.quantity)}. ${
          selected ? "Retirer le filtre" : "Filtrer les cartes"
        }. Ctrl+clic pour une sélection multiple`,
      );
      column.classList.toggle("is-selected", selected);
      column.classList.toggle("is-muted", selectedLabels.length > 0 && !selected);
      column.addEventListener("click", (event) => {
        onSelect(entry.label, event.ctrlKey || event.metaKey);
      });
    }
    const value = document.createElement("strong");
    value.textContent = formatCount(entry.quantity);
    const track = document.createElement("span");
    track.className = "chart-column-track";
    const fill = document.createElement("span");
    fill.className = "chart-column-fill";
    fill.style.height = `${Math.max((entry.quantity / maximum) * 100, 3)}%`;
    track.append(fill);
    const label = document.createElement("span");
    label.className = "chart-column-label";
    label.textContent = entry.label;
    column.append(value, track, label);
    plot.append(column);
  }

  scroll.append(plot);
  target.classList.toggle("chart-target--tooltip", showTooltip);
  target.replaceChildren(scroll, ...(showTooltip ? [tooltip] : []));
}

function renderFacetAnalysis(kind, entries) {
  const analysis = facetAnalyses[kind];
  analysis.entries = entries;
  analysis.filters.facet = analysis.filters.facet.filter((label) =>
    entries.some((entry) => entry.label === label),
  );
  const baseItems = filterFacetItems(searchableCards, kind, []);
  analysis.manaEntries = overviewDistribution(baseItems, "manaValues");
  analysis.filters.manaValues = analysis.filters.manaValues.filter((label) =>
    analysis.manaEntries.some((entry) => entry.label === label),
  );
  const filteredItems = filterFacetItems(
    searchableCards,
    kind,
    analysis.filters.facet,
    analysis.filters.manaValues,
  );
  const metrics = overviewMetrics(filteredItems);
  analysis.metrics.quantity.textContent = formatCount(metrics.totalQuantity);
  analysis.metrics.distinctNames.textContent = formatCount(metrics.distinctNames);
  analysis.metrics.value.textContent = formatCurrency(metrics.totalValueCents);
  renderDonutChart(kind, entries, analysis.target, {
    selectedLabels: analysis.filters.facet,
    showTooltip: true,
    onSelect: (label, additive) => {
      analysis.filters.facet = nextChartSelection(
        analysis.filters.facet,
        label,
        additive,
      );
      renderFacetAnalysis(kind, analysis.entries);
      focusChartValue(analysis.target, label);
    },
  });
  renderManaChart(analysis.manaEntries, analysis.manaTarget, {
    selectedLabels: analysis.filters.manaValues,
    onSelect: (label, additive) => {
      analysis.filters.manaValues = nextChartSelection(
        analysis.filters.manaValues,
        label,
        additive,
      );
      renderFacetAnalysis(kind, analysis.entries);
      focusChartValue(analysis.manaTarget, label);
    },
  });

  if (pageFromHash(window.location.hash) !== kind) return;

  renderAnalysisItems(
    filteredItems,
    analysis.rows,
    analysis.summary,
    "Aucune carte ne correspond à cette sélection.",
  );
  const filters = [];
  if (analysis.filters.facet.length > 0) {
    filters.push(
      analysis.filters.facet
        .map((label) => distributionLabel(kind, label))
        .join(" ou "),
    );
  }
  if (analysis.filters.manaValues.length > 0) {
    filters.push(`mana ${analysis.filters.manaValues.join(" ou ")}`);
  }
  if (filters.length > 0) analysis.summary.textContent += ` · ${filters.join(" · ")}`;
  analysis.reset.hidden = filters.length === 0;
}

function renderCharts(distributions) {
  for (const kind of facetKinds) {
    renderFacetAnalysis(kind, distributions[kind]);
  }
}

function deckInputValue(key) {
  const input = deckNumberInputs.get(key);
  const value = Number(input?.value ?? 0);
  if (!Number.isFinite(value)) return 0;
  return Math.min(Number(input.max), Math.max(Number(input.min), value));
}

function renderDeckBuilder() {
  const result = calculateBasicLands({
    basicLandSlots: deckInputValue("basicSlots"),
    multicolorLands: Object.fromEntries(
      DECK_COLOR_CODES.map((color) => [
        color,
        deckInputValue(`lands-${color}`),
      ]),
    ),
    coloredCards: Object.fromEntries(
      DECK_COLOR_CODES.map((color) => [
        color,
        deckInputValue(`cards-${color}`),
      ]),
    ),
  });
  const visibleTotal = DECK_COLOR_CODES.reduce(
    (total, color) => total + Math.max(0, result.recommendations[color]),
    0,
  );
  const segments = [];
  const legendItems = [];

  for (const [index, color] of DECK_COLOR_CODES.entries()) {
    const value = result.recommendations[color];
    const formatted = formatCount(Math.round(value * 10) / 10);
    if (value > 0) {
      const segment = document.createElement("span");
      segment.className = "deck-land-segment";
      segment.style.flexGrow = String(value);
      segment.style.background = chartColor("colors", color, index);
      segment.title = `${deckColorNames[color]} : ${formatted}`;
      segments.push(segment);
    }

    const legendItem = document.createElement("li");
    const label = document.createElement("span");
    const amount = document.createElement("strong");
    label.append(createColorSymbols([color]), deckColorNames[color]);
    amount.textContent = formatted;
    legendItem.append(label, amount);
    legendItems.push(legendItem);
  }

  deckLandStack.replaceChildren(...segments);
  deckLandStack.setAttribute(
    "aria-label",
    DECK_COLOR_CODES.map(
      (color) =>
        `${deckColorNames[color]} ${formatCount(
          Math.round(result.recommendations[color] * 10) / 10,
        )}`,
    ).join(", "),
  );
  deckLandLegend.replaceChildren(...legendItems);
  if (result.totalCards === 0) {
    deckLandMessage.textContent =
      "Renseignez au moins une carte avec un coût coloré.";
  } else if (
    DECK_COLOR_CODES.some((color) => result.recommendations[color] < 0)
  ) {
    deckLandMessage.textContent =
      "Une valeur négative indique que les terrains multicolores couvrent déjà cette couleur.";
  } else {
    deckLandMessage.textContent = `${formatCount(
      deckInputValue("basicSlots"),
    )} terrains de base à répartir · ${formatCount(
      result.totalCards,
    )} contributions colorées.`;
  }
  deckLandStack.classList.toggle("is-empty", visibleTotal === 0);
}

function renderOverviewAnalysis() {
  if (!overviewDashboard) return;
  const hasFilters = Object.values(overviewChartFilters).some(
    (selected) => selected.length > 0,
  );
  const filteredItems = filterOverviewItems(
    searchableCards,
    overviewChartFilters,
  );
  const metrics = hasFilters
    ? overviewMetrics(filteredItems)
    : overviewDashboard.metrics;
  renderOverviewMetrics(metrics);
  renderMostExpensiveCard(
    hasFilters
      ? mostExpensiveOverviewItem(filteredItems)
      : overviewDashboard.mostExpensiveCard,
  );

  for (const kind of ["colors", "types", "manaValues"]) {
    const target = chartTargets[kind][0];
    const entries = overviewDistribution(
      filterOverviewItems(searchableCards, overviewChartFilters, kind),
      kind,
    );
    const options = {
      selectedLabels: overviewChartFilters[kind],
      showTooltip: true,
      onSelect: (label, additive) => {
        overviewChartFilters[kind] = nextChartSelection(
          overviewChartFilters[kind],
          label,
          additive,
        );
        renderOverviewAnalysis();
        focusChartValue(target, label);
      },
    };
    if (kind === "manaValues") renderManaChart(entries, target, options);
    else renderDonutChart(kind, entries, target, options);
  }

  resetOverviewChartFilters.hidden = !hasFilters;
  overviewFilterSummary.textContent = hasFilters
    ? `${formatCount(metrics.totalQuantity)} carte${metrics.totalQuantity > 1 ? "s" : ""} dans la sélection`
    : "Cliquez sur un graphique pour filtrer toute la vue.";
}

function renderTypeOptions(entries) {
  const selected = typeSelect.value;
  const selectableEntries = selectableTypeEntries(entries);
  typeSelect.replaceChildren();
  for (const entry of selectableEntries) {
    const option = document.createElement("option");
    option.value = entry.label;
    option.textContent = `${distributionLabel("types", entry.label)} · ${formatCount(entry.quantity)}`;
    typeSelect.append(option);
  }

  if (selectableEntries.length === 0) {
    const option = document.createElement("option");
    option.value = "";
    option.textContent = "Aucun type disponible";
    typeSelect.append(option);
    typeSelect.disabled = true;
    return;
  }

  if (selectableEntries.some((entry) => entry.label === selected)) {
    typeSelect.value = selected;
  }
  typeSelect.disabled = false;
}

function renderAnalysisItems(items, rowsTarget, summaryTarget, emptyMessage) {
  hideCardPreview();
  analysisContexts.set(rowsTarget, { items, summaryTarget, emptyMessage });
  rowsTarget.replaceChildren();
  const groupedItems = groupCardPrintings(items);
  summaryTarget.textContent = `${formatCount(groupedItems.length)} impression${groupedItems.length > 1 ? "s" : ""}`;
  if (groupedItems.length === 0) {
    const row = document.createElement("tr");
    const cell = document.createElement("td");
    cell.colSpan = 5;
    cell.className = "empty-state";
    cell.textContent = emptyMessage;
    row.append(cell);
    rowsTarget.append(row);
    return;
  }

  for (const item of sortCards(groupedItems, sortStateFor(rowsTarget))) {
    const row = document.createElement("tr");
    const nameCell = document.createElement("td");
    const detailButton = document.createElement("button");
    detailButton.type = "button";
    detailButton.className = "card-detail-trigger";
    detailButton.textContent = item.name;
    detailButton.setAttribute("aria-label", `Consulter la fiche de ${item.name}`);
    detailButton.addEventListener("click", () => showCardDetail(item));
    attachCardPreview(detailButton, item);
    nameCell.append(detailButton);
    row.append(nameCell);
    appendCell(row, item.setCode);
    appendCell(row, formatCount(item.quantity), "numeric");
    appendCell(row, createColorSymbols(item.colors));
    appendCell(
      row,
      item.manaValue === null ? "Indisponible" : String(item.manaValue),
      "numeric",
    );
    rowsTarget.append(row);
  }
}

function renderTypeItems(items) {
  renderAnalysisItems(
    items,
    typeRows,
    typeResultSummary,
    "Aucune carte pour ce type.",
  );
}

function clearTypeChartFilters() {
  typeChartFilters.colors = [];
  typeChartFilters.manaValues = [];
}

function focusChartValue(target, label) {
  window.requestAnimationFrame(() => {
    const control = [...target.querySelectorAll("[data-chart-value]")].find(
      (element) => element.dataset.chartValue === label,
    );
    control?.focus();
  });
}

function renderTypeAnalysisResults() {
  if (!typeAnalysis) return;
  renderDonutChart("colors", typeAnalysis.colors, typeChartTargets.colors, {
    selectedLabels: typeChartFilters.colors,
    showTooltip: true,
    onSelect: (label, additive) => {
      typeChartFilters.colors = nextChartSelection(
        typeChartFilters.colors,
        label,
        additive,
      );
      renderTypeAnalysisResults();
      focusChartValue(typeChartTargets.colors, label);
    },
  });
  renderManaChart(typeAnalysis.manaValues, typeChartTargets.manaValues, {
    selectedLabels: typeChartFilters.manaValues,
    onSelect: (label, additive) => {
      typeChartFilters.manaValues = nextChartSelection(
        typeChartFilters.manaValues,
        label,
        additive,
      );
      renderTypeAnalysisResults();
      focusChartValue(typeChartTargets.manaValues, label);
    },
  });

  const filters = [];
  if (typeChartFilters.colors.length > 0) {
    filters.push(
      typeChartFilters.colors
        .map((color) => distributionLabel("colors", color))
        .join(" ou "),
    );
  }
  if (typeChartFilters.manaValues.length > 0) {
    filters.push(`mana ${typeChartFilters.manaValues.join(" ou ")}`);
  }
  const items = filterTypeItems(typeAnalysis.items, typeChartFilters);
  const metrics = overviewMetrics(items);
  document.querySelector("#type-metric-quantity").textContent = formatCount(
    metrics.totalQuantity,
  );
  document.querySelector("#type-metric-distinct-names").textContent = formatCount(
    metrics.distinctNames,
  );
  document.querySelector("#type-metric-value").textContent = formatCurrency(
    metrics.totalValueCents,
  );
  renderTypeItems(items);
  if (filters.length > 0) {
    typeResultSummary.textContent += ` · ${filters.join(" · ")}`;
  }
  resetTypeChartFilters.hidden = filters.length === 0;
  typeAnalysisStatus.textContent = filters.length
    ? `${formatCount(items.length)} impressions affichées, filtres ${filters.join(", ")}`
    : `Analyse ${typeAnalysis.selectedType} chargée`;
}

function setTypeAnalysisLoading() {
  typeAnalysis = undefined;
  clearTypeChartFilters();
  resetTypeChartFilters.hidden = true;
  typeSelect.disabled = true;
  typeAnalysisError.hidden = true;
  typeAnalysisStatus.textContent = "Actualisation de l’analyse par type";
  document.querySelector("#type-metric-quantity").textContent = "…";
  document.querySelector("#type-metric-distinct-names").textContent = "…";
  document.querySelector("#type-metric-value").textContent = "…";
  typeResultSummary.textContent = "Chargement…";
  renderChartMessage(typeChartTargets.colors, "Chargement…");
  renderChartMessage(typeChartTargets.manaValues, "Chargement…");
  typeRows.innerHTML = '<tr><td colspan="6" class="empty-state">Chargement des cartes…</td></tr>';
}

async function loadTypeAnalysis() {
  const cardType = typeSelect.value;
  if (!cardType) {
    typeAnalysis = undefined;
    clearTypeChartFilters();
    resetTypeChartFilters.hidden = true;
    renderChartEmpty(typeChartTargets.colors);
    renderChartEmpty(typeChartTargets.manaValues);
    renderTypeItems([]);
    return;
  }

  setTypeAnalysisLoading();
  try {
    if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
    const analysis = await invoke("load_type_analysis", { cardType });
    typeAnalysis = analysis;
    renderTypeAnalysisResults();
  } catch (error) {
    typeAnalysis = undefined;
    typeAnalysisError.textContent = String(error);
    typeAnalysisError.hidden = false;
    renderChartMessage(typeChartTargets.colors, "Analyse indisponible.");
    renderChartMessage(typeChartTargets.manaValues, "Analyse indisponible.");
    renderTypeItems([]);
  } finally {
    typeSelect.disabled = typeSelect.options.length === 0;
  }
}

function renderDistributions(distributions) {
  for (const kind of Object.keys(distributionLists)) {
    renderDistribution(kind, distributions[kind]);
  }
  renderCharts(distributions);
}

function resetImportSource() {
  selectedImportFile = undefined;
  fileInput.value = "";
  collectionUrlInput.value = "";
  collectionFileStatus.textContent = "Aucun fichier sélectionné.";
  collectionDropZone.classList.remove("is-dragging");
}

function selectImportFile(file) {
  if (!file) return;
  selectedImportFile = file;
  collectionUrlInput.value = "";
  collectionFileStatus.textContent = file.name;
  clearImportError();
}

function setCollectionImportMode(collection) {
  collectionUpdateTarget = collection;
  importTitle.textContent = collection
    ? `Mettre à jour « ${collection.name} »`
    : "Ajouter une collection";
  collectionNameInput.value = collection?.name ?? "";
  cancelCollectionUpdateButton.hidden = !collection;
  resetImportSource();
  clearImportError();
  importResult.hidden = true;
  setImportLoading(false);
  if (collection) {
    importTitle.scrollIntoView({ block: "center" });
    fileInput.focus();
  }
}

function showManagedCollectionsError(message) {
  managedCollectionsError.textContent = message;
  managedCollectionsError.hidden = false;
}

function clearManagedCollectionsError() {
  managedCollectionsError.textContent = "";
  managedCollectionsError.hidden = true;
}

function renderManagedCollections() {
  managedCollectionsList.replaceChildren();
  if (managedCollections.length === 0) {
    const empty = document.createElement("p");
    empty.className = "managed-collections-empty";
    empty.textContent = "Aucune collection enregistrée.";
    managedCollectionsList.append(empty);
    return;
  }

  for (const collection of managedCollections) {
    const row = document.createElement("article");
    row.className = "managed-collection";
    row.setAttribute("role", "listitem");

    const choice = document.createElement("label");
    choice.className = "managed-collection-choice";
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = collection.selected;
    const copy = document.createElement("span");
    const name = document.createElement("strong");
    name.className = "managed-collection-name";
    name.textContent = collection.name;
    const meta = document.createElement("span");
    meta.className = "managed-collection-meta";
    meta.textContent = `${formatCount(collection.totalQuantity)} carte${collection.totalQuantity > 1 ? "s" : ""}`;
    copy.append(name, meta);
    choice.append(checkbox, copy);

    const actions = document.createElement("div");
    actions.className = "managed-collection-actions";
    const rename = document.createElement("button");
    rename.type = "button";
    rename.textContent = "Renommer";
    const update = document.createElement("button");
    update.type = "button";
    update.textContent = "Mettre à jour";
    actions.append(rename, update);
    row.append(choice, actions);
    managedCollectionsList.append(row);

    checkbox.addEventListener("change", async () => {
      checkbox.disabled = true;
      clearManagedCollectionsError();
      try {
        if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
        await invoke("set_collection_selected", {
          collectionId: collection.id,
          selected: checkbox.checked,
        });
        collection.selected = checkbox.checked;
        managedCollectionsStatus.textContent = `${collection.name} ${checkbox.checked ? "incluse" : "masquée"}`;
        await loadDashboard(true);
      } catch (error) {
        checkbox.checked = !checkbox.checked;
        showManagedCollectionsError(String(error));
      } finally {
        checkbox.disabled = false;
      }
    });

    rename.addEventListener("click", async () => {
      const value = window.prompt("Nouveau nom de la collection", collection.name);
      if (value === null || value.trim() === collection.name) return;
      rename.disabled = true;
      clearManagedCollectionsError();
      try {
        if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
        await invoke("rename_collection", {
          collectionId: collection.id,
          name: value.trim(),
        });
        if (collectionUpdateTarget?.id === collection.id) {
          collectionUpdateTarget = { ...collectionUpdateTarget, name: value.trim() };
          importTitle.textContent = `Mettre à jour « ${value.trim()} »`;
          collectionNameInput.value = value.trim();
        }
        await loadManagedCollections();
      } catch (error) {
        showManagedCollectionsError(String(error));
      } finally {
        rename.disabled = false;
      }
    });

    update.addEventListener("click", () => setCollectionImportMode(collection));
  }
}

async function loadManagedCollections() {
  clearManagedCollectionsError();
  try {
    if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
    managedCollections = await invoke("list_collections");
    renderManagedCollections();
  } catch (error) {
    showManagedCollectionsError(String(error));
  }
}

function renderImportResult(summary) {
  importSummary.textContent = formatImportSummary(summary);
  importIssueList.replaceChildren();
  for (const issue of summary.issues) {
    const item = document.createElement("li");
    item.textContent = `Ligne ${formatCount(issue.line)} — ${issue.reason}`;
    importIssueList.append(item);
  }
  importIssues.hidden = summary.issues.length === 0;
  importResult.hidden = false;
}

function diagnosticEmpty(message) {
  const item = document.createElement("li");
  item.className = "diagnostic-empty";
  item.textContent = message;
  return item;
}

function renderDiagnostics(diagnostics) {
  diagnosticsSummary.textContent = formatDiagnosticsSummary(diagnostics);
  invalidCount.textContent = formatCount(diagnostics.invalidRows.length);
  unresolvedCount.textContent = formatCount(diagnostics.unresolvedRows.length);
  invalidDiagnostics.replaceChildren();
  unresolvedDiagnostics.replaceChildren();

  if (diagnostics.invalidRows.length === 0) {
    invalidDiagnostics.append(diagnosticEmpty("Aucune ligne invalide."));
  }
  for (const issue of diagnostics.invalidRows) {
    const item = document.createElement("li");
    item.textContent = `Ligne ${formatCount(issue.line)} — ${issue.reason}`;
    invalidDiagnostics.append(item);
  }

  if (diagnostics.unresolvedRows.length === 0) {
    unresolvedDiagnostics.append(diagnosticEmpty("Aucune carte à vérifier."));
  }
  for (const row of diagnostics.unresolvedRows) {
    const item = document.createElement("li");
    const title = document.createElement("strong");
    title.textContent = row.name;
    const metadata = document.createElement("p");
    const language = row.language ? ` · langue ${row.language}` : "";
    const printing = row.printing ? ` · finition ${row.printing}` : "";
    metadata.textContent = `${row.setCode} · ${row.collectorNumber} · quantité ${formatCount(row.quantity)}${language}${printing} · ${statusLabel(row.status)}`;
    item.append(title, metadata);
    if (row.candidateUuids.length > 0) {
      const candidates = document.createElement("p");
      candidates.className = "diagnostic-candidates";
      candidates.textContent = `Candidats : ${row.candidateUuids.join(", ")}`;
      item.append(candidates);
    }
    unresolvedDiagnostics.append(item);
  }
}

async function loadDashboard(refreshSearchIndex = false) {
  clearError();
  setLoading(true);
  statusRegion.textContent = "Chargement de la collection";
  try {
    if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
    const query = currentQuery();
    const dashboard = await invoke("load_dashboard", { query });
    if (!hasActiveQuery(query)) {
      updateSearchableCards(dashboard.items);
    } else if (refreshSearchIndex) {
      const unfiltered = await invoke("load_dashboard", {
        query: { search: "", colors: [], rarities: [] },
      });
      updateSearchableCards(unfiltered.items);
    }
    renderMetrics(dashboard.metrics, dashboard.items.length);
    overviewDashboard = {
      metrics: dashboard.metrics,
      mostExpensiveCard: dashboard.mostExpensiveCard,
    };
    renderDistributions(dashboard.distributions);
    renderOverviewAnalysis();
    renderTypeOptions(dashboard.distributions.types);
    await loadTypeAnalysis();
    renderItems(dashboard.items);
    renderDiagnostics(dashboard.diagnostics);
    statusRegion.textContent = "Collection chargée";
    return true;
  } catch (error) {
    showError(String(error));
    return false;
  } finally {
    setLoading(false);
  }
}

form.addEventListener("submit", (event) => {
  event.preventDefault();
  loadDashboard();
});

collectionSearch.addEventListener("input", refreshNameSuggestions);
collectionSearch.addEventListener("focus", refreshNameSuggestions);
cardDetailSearch.addEventListener("input", () => {
  cardDetailSearchError.hidden = true;
  refreshDetailSuggestions();
});
cardDetailSearch.addEventListener("focus", refreshDetailSuggestions);
cardDetailSearchForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const cards = findCardsByName(searchableCards, cardDetailSearch.value);
  if (cards.length === 0) {
    cardDetailSearchError.textContent = "Aucune carte de la collection ne correspond à cette recherche.";
    cardDetailSearchError.hidden = false;
    return;
  }
  showCardDetail(cards[0]);
});

resetOverviewChartFilters.addEventListener("click", () => {
  overviewChartFilters.colors = [];
  overviewChartFilters.types = [];
  overviewChartFilters.manaValues = [];
  renderOverviewAnalysis();
  chartTargets.colors[0].querySelector("[data-chart-value]")?.focus();
});

typeSelect.addEventListener("change", loadTypeAnalysis);
resetTypeChartFilters.addEventListener("click", () => {
  clearTypeChartFilters();
  renderTypeAnalysisResults();
  typeSelect.focus();
});
for (const kind of facetKinds) {
  const analysis = facetAnalyses[kind];
  analysis.reset.addEventListener("click", () => {
    analysis.filters.facet = [];
    analysis.filters.manaValues = [];
    renderFacetAnalysis(kind, analysis.entries);
    const firstLabel = analysis.entries[0]?.label;
    if (firstLabel) focusChartValue(analysis.target, firstLabel);
  });
}

fileInput.addEventListener("change", () => {
  selectImportFile(fileInput.files?.item(0));
});

collectionUrlInput.addEventListener("input", () => {
  if (!collectionUrlInput.value.trim()) return;
  selectedImportFile = undefined;
  fileInput.value = "";
  collectionFileStatus.textContent = "Aucun fichier sélectionné.";
  clearImportError();
});

for (const eventName of ["dragenter", "dragover"]) {
  collectionDropZone.addEventListener(eventName, (event) => {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
    collectionDropZone.classList.add("is-dragging");
  });
}
collectionDropZone.addEventListener("dragleave", () => {
  collectionDropZone.classList.remove("is-dragging");
});
collectionDropZone.addEventListener("drop", (event) => {
  event.preventDefault();
  collectionDropZone.classList.remove("is-dragging");
  selectImportFile(event.dataTransfer?.files?.item(0));
});

cancelCollectionUpdateButton.addEventListener("click", () => {
  setCollectionImportMode(undefined);
  collectionNameInput.focus();
});

importForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  clearImportError();
  importResult.hidden = true;
  const name = collectionNameInput.value.trim();
  const url = collectionUrlInput.value.trim();
  if (!collectionUpdateTarget && !name) {
    showImportError("Donnez un nom à la collection.");
    collectionNameInput.focus();
    return;
  }
  if (!selectedImportFile && !url) {
    showImportError("Choisissez, déposez ou indiquez un export CSV DragonShield.");
    return;
  }

  setImportLoading(true);
  statusRegion.textContent = "Import de la collection";
  try {
    if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
    const csv = selectedImportFile
      ? await selectedImportFile.text()
      : await invoke("download_collection_csv", { url });
    const summary = collectionUpdateTarget
      ? await invoke("update_collection", {
          collectionId: collectionUpdateTarget.id,
          csv,
        })
      : await invoke("create_collection", { name, csv });
    renderImportResult(summary);
    setCollectionImportMode(undefined);
    importResult.hidden = false;
    await loadManagedCollections();
    if (await loadDashboard(true)) statusRegion.textContent = "Import terminé";
  } catch (error) {
    showImportError(String(error));
  } finally {
    setImportLoading(false);
  }
});

function todayLocalIso() {
  const today = new Date();
  const year = today.getFullYear();
  const month = String(today.getMonth() + 1).padStart(2, "0");
  const day = String(today.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

async function refreshReferenceData(force) {
  clearReferenceError();
  referenceResult.hidden = true;
  setReferenceLoading(true);
  statusRegion.textContent = "Actualisation des données de référence";
  try {
    if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
    const summary = await invoke("update_reference_data", {
      localDate: todayLocalIso(),
      force,
    });
    referenceResult.textContent = formatReferenceUpdate(summary);
    referenceResult.hidden = false;
    if (summary.status === "updated" && (await loadDashboard(true))) {
      statusRegion.textContent = "Données de référence actualisées";
    } else if (summary.status === "current") {
      statusRegion.textContent = "Données de référence à jour";
    }
  } catch (error) {
    await loadDashboard(true);
    showReferenceError(String(error));
  } finally {
    setReferenceLoading(false);
  }
}

referenceButton.addEventListener("click", () => {
  refreshReferenceData(true);
});

clearImageCacheButton.addEventListener("click", async () => {
  imageCacheError.hidden = true;
  clearImageCacheButton.disabled = true;
  clearImageCacheButton.textContent = "Vidage…";
  try {
    if (!invoke) throw new Error("L’interface doit être ouverte depuis l’application DicoMTG.");
    renderImageCacheStats(await invoke("clear_image_cache"));
    statusRegion.textContent = "Cache des illustrations vidé";
  } catch (error) {
    imageCacheError.textContent = String(error);
    imageCacheError.hidden = false;
    clearImageCacheButton.disabled = false;
  } finally {
    clearImageCacheButton.textContent = "Vider le cache";
  }
});

for (const [key, input] of deckNumberInputs) {
  input.addEventListener("input", () => {
    const range = deckRangeInputs.get(key);
    input.value = String(deckInputValue(key));
    if (range) range.value = input.value;
    renderDeckBuilder();
  });
}
for (const [key, input] of deckRangeInputs) {
  input.addEventListener("input", () => {
    deckNumberInputs.get(key).value = input.value;
    renderDeckBuilder();
  });
}

window.addEventListener("hashchange", routeFromLocation);
window.addEventListener("resize", hideCardPreview);
window.addEventListener("scroll", hideCardPreview, true);
window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") hideCardPreview();
});
for (const target of document.querySelectorAll("[data-mana-symbol]")) {
  target.replaceWith(createManaSymbolText(target.dataset.manaSymbol));
}
initializeSortableTables();
renderDeckBuilder();
routeFromLocation();
setCollectionImportMode(undefined);
loadManagedCollections();
loadDashboard(true).then((loaded) => {
  if (loaded) refreshReferenceData(false);
});
