import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext } from "node:vm";

import { overviewMetrics } from "./charts.js";
import { cardsTableCsv } from "./csv-export.js";
import { PAGE_IDS, pageFromHash } from "./navigation.js";

test("exposes the Power BI views and the local data workspace", () => {
  assert.deepEqual(PAGE_IDS, [
    "overview",
    "card",
    "types",
    "keywords",
    "sets",
    "rarities",
    "collection",
    "deck",
    "data",
  ]);
});

test("resolves known hashes and falls back safely to the overview", () => {
  assert.equal(pageFromHash("#types"), "types");
  assert.equal(pageFromHash("#collection"), "collection");
  assert.equal(pageFromHash("#deck"), "deck");
  assert.equal(pageFromHash(""), "overview");
  assert.equal(pageFromHash("#unknown"), "overview");
});

test("renders one navigation entry and one page for every route", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const pages = [...html.matchAll(/data-page="([^"]+)"/g)].map((match) => match[1]);
  const links = [...html.matchAll(/data-nav-page="([^"]+)"/g)].map(
    (match) => match[1],
  );

  assert.deepEqual(pages, PAGE_IDS);
  assert.deepEqual(links, PAGE_IDS);
});

test("uses French copy for every static page heading", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");

  for (const englishCopy of [
    "Deck Builder",
    "Statistics",
    "Card view",
    "Keyword",
    "Set",
    "Rarity",
    "List of cards",
    "Help Deck Builder",
  ]) {
    assert.equal(html.includes(englishCopy), false, englishCopy);
  }
  assert.doesNotMatch(html, /class="eyebrow"/);
});

test("keeps the navigation visible without the decorative header", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const styles = readFileSync(new URL("./styles.css", import.meta.url), "utf8");

  assert.doesNotMatch(html, /class="app-header"/);
  assert.match(styles, /\.app-nav\s*{[^}]*position: fixed;/s);
});

test("renders the Power BI chart subjects on their analysis pages", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const charts = [...html.matchAll(/data-chart="([^"]+)"/g)].map(
    (match) => match[1],
  );

  assert.deepEqual(
    Object.fromEntries(
      ["colors", "types", "keywords", "sets", "rarities", "manaValues"].map(
        (kind) => [kind, charts.filter((chart) => chart === kind).length],
      ),
    ),
    {
      colors: 1,
      types: 1,
      keywords: 1,
      sets: 1,
      rarities: 1,
      manaValues: 1,
    },
  );
  assert.deepEqual(
    [...html.matchAll(/data-type-chart="([^"]+)"/g)].map((match) => match[1]),
    ["colors", "manaValues"],
  );
  assert.match(html, /id="reset-type-chart-filters"/);
});

test("recreates the Power BI deck-building helper", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");

  assert.match(html, /data-nav-page="deck"/);
  assert.match(html, /data-page="deck"/);
  assert.equal((html.match(/data-deck-number=/g) ?? []).length, 11);
  assert.equal((html.match(/data-deck-range=/g) ?? []).length, 11);
  assert.match(html, /id="deck-land-stack"/);
  assert.match(html, /id="deck-land-legend"/);
  assert.match(script, /calculateBasicLands/);
  assert.match(script, /function renderDeckBuilder\(\)/);
});

test("reserves an accessible printing gallery and overview image", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const config = JSON.parse(
    readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"),
  );

  assert.match(html, /id="card-detail-printings"/);
  assert.match(html, /id="card-detail-rules"/);
  assert.match(html, /id="most-expensive-card"/);
  assert.match(html, /id="most-expensive-image"/);
  assert.match(html, /id="most-expensive-image-state"[^>]*aria-live="polite"/);
  assert.doesNotMatch(html, /id="most-expensive-(?:meta|name)"/);
  assert.match(
    readFileSync(new URL("./app.js", import.meta.url), "utf8"),
    /showCardImage\(item, mostExpensiveImage, mostExpensiveImageState\);/,
  );
  assert.match(config.app.security.csp, /img-src[^;]*blob:/);
  assert.match(config.app.security.csp, /img-src[^;]*https:\/\/api\.scryfall\.com/);
  assert.match(config.app.security.csp, /img-src[^;]*https:\/\/cards\.scryfall\.io/);
  assert.match(html, /rel="preconnect" href="https:\/\/cards\.scryfall\.io"/);
});

test("switches double-faced card images only from the detail control", () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const styles = readFileSync(new URL("./styles.css", import.meta.url), "utf8");

  assert.match(script, /item\.hasBackFace/);
  assert.match(script, /face === "front" \? "back" : "front"/);
  assert.match(script, /load_card_image_bytes[\s\S]*face/);
  assert.match(
    styles,
    /\.card-detail-flip \{[^}]*width: 34px;[^}]*height: 34px;[^}]*min-height: 34px;[^}]*border-radius: 50%;/s,
  );
});

test("uses the Prawse publisher and application identifier", () => {
  const config = JSON.parse(
    readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"),
  );

  assert.equal(config.identifier, "fr.prawse.dicomtg");
  assert.equal(config.app.appDirectoriesOverride, "./");
  assert.equal(config.bundle.publisher, "Prawse");
  assert.equal(config.bundle.windows.nsis.installerIcon, "icons/dicomtg.ico");
  assert.equal(config.bundle.windows.nsis.uninstallerIcon, "icons/dicomtg.ico");
});

test("exposes bounded image-cache controls and an accessible card preview", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");

  assert.match(html, /id="image-cache-usage"/);
  assert.match(html, /id="clear-image-cache"/);
  assert.match(html, /id="card-preview"[^>]*role="tooltip"/);
  assert.match(html, /id="card-preview-image-state"[^>]*aria-live="polite"/);
});

test("supports named collections from a picker, drop zone, or public URL", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");

  assert.match(html, /id="managed-collections"/);
  assert.match(html, /id="collection-name"/);
  assert.match(html, /id="collection-drop-zone"/);
  assert.match(html, /id="collection-url"[^>]*type="url"/s);
  assert.match(script, /dragover/);
  assert.match(script, /drop/);
  assert.match(script, /set_collection_selected/);
  assert.doesNotMatch(html, /id="collection-reimport/);
  assert.match(script, /window\.confirm\([\s\S]*?invoke\("delete_collection"/);
  assert.doesNotMatch(script, /window\.prompt|invoke\("rename_collection"/);
  assert.match(script, /download_collection_csv/);
});

test("prefills the existing collection name and remembered Drive link for editing", () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const context = {
    collectionUpdateTarget: undefined,
    importTitle: { scrollIntoView() {} },
    collectionNameInput: { focus() {} },
    collectionUrlInput: {},
    cancelCollectionUpdateButton: {},
    importResult: {},
    resetImportSource() { context.collectionUrlInput.value = ""; },
    clearImportError() {},
    setImportLoading() {},
    collection: { name: "Ma collection", driveUrl: "https://drive.google.com/open?id=abc" },
  };
  const start = script.indexOf("function setCollectionImportMode(");
  runInNewContext(script.slice(start, script.indexOf("\nfunction ", start + 1)) +
    "\nsetCollectionImportMode(collection);", context);
  assert.equal(context.collectionNameInput.value, context.collection.name);
  assert.equal(context.collectionUrlInput.value, context.collection.driveUrl);
});

test("reimports from the prefilled Drive link and saves only the name when cleared", async () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const start = script.indexOf('importForm.addEventListener("submit"');
  const submit = script.slice(start, script.indexOf("\nfunction ", start + 1));
  const link = "https://drive.google.com/open?id=abc";
  for (const url of [link, ""]) {
    const calls = [];
    const context = {
      importForm: { addEventListener(_event, handler) { context.submit = handler; } },
      importButton: {},
      importInProgress: false,
      appUpdater: { installing: false },
      collectionNameInput: { value: "Nouveau nom" },
      collectionUpdateTarget: { id: 4, driveUrl: link },
      selectedImportFile: undefined,
      collectionUrlInput: { value: url },
      importResult: {}, importSummary: {}, importIssues: {}, statusRegion: {},
      collectionSnapshotCurrent: true,
      clearImportError() {},
      showImportError: assert.fail,
      setImportLoading() {},
      async invoke(command, args) {
        calls.push(command);
        if (command === "download_collection_csv") {
          assert.equal(args.url, link);
          return "csv";
        }
        assert.equal(args.collectionId, 4);
        assert.equal(args.name, "Nouveau nom");
        assert.equal(args.csv, url ? "csv" : null);
        assert.equal(args.sourceUrl, url || null);
        return url ? {} : null;
      },
      async loadManagedCollections() {},
      async loadDashboard() { return true; },
      setCollectionImportMode() {},
      renderImportResult() {},
    };
    runInNewContext(submit, context);
    await context.submit({ preventDefault() {} });
    assert.deepEqual(calls, url ? ["download_collection_csv", "update_collection"] : ["update_collection"]);
  }
});

test("offers assisted card searches without displaying the image provider", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");

  assert.match(html, /id="search"[^>]*list="card-name-suggestions"/s);
  assert.match(html, /id="card-detail-search"/);
  assert.match(html, /id="card-detail-search"[^>]*list="card-detail-suggestions"/s);
  assert.match(html, /id="card-detail-suggestions"/);
  assert.match(html, /id="card-detail-search-error"[^>]*role="alert"/s);
  assert.doesNotMatch(html, /Image fournie par Scryfall/);
  assert.doesNotMatch(html, /Cardmarket/);
});

test("groups the card detail by name and removes printing-specific facts", () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");

  assert.match(script, /groupCardPrintings/);
  assert.match(script, /load_card_localization/);
  assert.doesNotMatch(script, /Finition possédée|\["UUID"|\["Type"|\["Finitions"/);
  assert.doesNotMatch(script, /\["Langue"|Prix de référence|Date du prix/);
});

test("renders card colors and mana costs with official symbol images", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const config = JSON.parse(
    readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"),
  );

  assert.match(html, /rel="preconnect" href="https:\/\/svgs\.scryfall\.io"/);
  assert.match(config.app.security.csp, /img-src[^;]*https:\/\/svgs\.scryfall\.io/);
  assert.match(script, /createManaSymbolText/);
  assert.doesNotMatch(script, /item\.colors\.join/);
  assert.doesNotMatch(html, /\/>&nbsp;?[WUBRG]<\/label>/);
});

test("previews cards from both table layouts using the shared card row", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");

  assert.doesNotMatch(html, /Clic · Ctrl\+clic multiple/);
  assert.match(script, /function createCardRow\(item\)[\s\S]*?attachCardPreview\(detailButton, item\)/);
  assert.match(script, /function createCollectionRow\(item\) \{\s+const row = createCardRow\(item\)/);
  assert.match(script, /function createAnalysisRow\(item\) \{\s+const row = createCardRow\(item\)/);
});

test("exports every sorted result including rows outside the virtual window", async () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  assert.deepEqual(
    [...html.matchAll(/data-export-table="([^"]+)"/g)].map((match) => match[1]),
    ["type-analysis-rows", "keywords-analysis-rows", "sets-analysis-rows", "rarities-analysis-rows", "collection-rows"],
  );
  const target = {
    closest: (selector) => selector === "table"
      ? { dataset: { sortKeys: "name,quantity" } }
      : { dataset: { pageTitle: "Types" } },
  };
  const items = [{ name: "Zeta", quantity: 2 }, { name: "Alpha", quantity: 1 }];
  const virtualTables = new Map([[target, { items, visible: new Map([[0, {}]]) }]]);
  let file;
  let clicked = false;
  let revoked;
  const link = { click() { clicked = true; }, remove() {} };
  const start = script.indexOf("function exportTableCsv(");
  runInNewContext(script.slice(start, script.indexOf("\nfunction ", start + 1)) + "\nexportTableCsv(target);", {
    target, virtualTables, cardsTableCsv, Blob,
    URL: { createObjectURL(blob) { file = blob; return "blob:test"; }, revokeObjectURL(url) { revoked = url; } },
    document: { createElement: () => link, body: { append() {} } },
    window: { setTimeout(callback) { callback(); } },
  });
  assert.equal(clicked, true);
  assert.equal(link.download, "DicoMTG-Types.csv");
  assert.equal(revoked, "blob:test");
  assert.equal(Buffer.from(await file.arrayBuffer()).toString(), "\uFEFFCarte;Qté\r\nZeta;2\r\nAlpha;1\r\n");
});

test("uses the shared virtual window for Types on reopen and scroll", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  assert.match(html, /class="table-shell virtual-table"[^>]*aria-label="Cartes du type sélectionné"/);
  const calls = [];
  const typeRows = {};
  const context = {
    typeRows,
    typeAnalysisPending: false,
    facetAnalyses: {},
    pageFromHash,
    window: { location: { hash: "#types" }, requestAnimationFrame: (fn) => { context.tick = fn; } },
    collectionFrame: undefined,
    renderVirtualWindow: (target) => calls.push(target),
  };
  const active = script.slice(script.indexOf("function renderActiveCollectionPage("), script.indexOf("function navigateToPage("));
  const scroll = script.slice(script.indexOf("function scheduleVirtualWindow("), script.indexOf("function createCardRow("));
  runInNewContext(`${active}\n${scroll}\nrenderActiveCollectionPage("types"); scheduleVirtualWindow();`, context);
  context.tick();
  assert.deepEqual(calls, [typeRows, typeRows]);
});

test("reuses facet tables until the complete collection snapshot changes", () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const calls = [];
  const summary = { entries: [], rows: { replaceChildren() { calls.push("invalidate"); } } };
  const context = {
    facetAnalyses: { sets: summary },
    analysisContexts: new WeakMap(),
    virtualTables: new WeakMap(),
    refreshNameSuggestions() {},
    refreshDetailSuggestions() {},
    renderFacetAnalysis() { calls.push("first"); summary.baseItems = []; },
    renderVirtualWindow() { calls.push("reopen"); },
  };
  const active = script.slice(
    script.indexOf("function renderActiveCollectionPage("),
    script.indexOf("function navigateToPage("),
  );
  const updateStart = script.indexOf("function updateSearchableCards(");
  const update = script.slice(updateStart, script.indexOf("\nfunction ", updateStart + 1));
  runInNewContext(`${active}\n${update}
    renderActiveCollectionPage("sets");
    renderActiveCollectionPage("sets");
    updateSearchableCards([]);
    renderActiveCollectionPage("sets");
  `, context);
  assert.deepEqual(calls, ["first", "reopen", "invalidate", "first"]);
});

test("cross-filters every overview widget from the interactive charts", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");

  assert.match(html, /id="reset-overview-chart-filters"/);
  assert.match(html, /id="overview-filter-summary"[^>]*role="status"/s);
  assert.match(script, /function renderOverviewAnalysis\(\)/);
  assert.match(script, /filterOverviewItems/);
  assert.match(script, /overviewMetrics/);
  assert.match(script, /mostExpensiveOverviewItem/);
  assert.match(script, /selectedLabels: overviewChartFilters\[kind\]/);
  assert.match(script, /onSelect: \(label, additive\)/);
});

test("places the vertical collection highlight beside the four overview metrics", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const styles = readFileSync(new URL("./styles.css", import.meta.url), "utf8");
  const overview = html.slice(0, html.indexOf('data-page="card"'));
  const data = html.slice(html.indexOf('data-page="data"'));

  assert.match(
    html,
    /class="overview-summary"[\s\S]*class="metric-grid"[\s\S]*class="most-expensive-section"/,
  );
  assert.match(
    styles,
    /\.overview-summary \{[^}]*grid-template-columns: minmax\(320px, 380px\)/s,
  );
  assert.match(
    styles,
    /\.overview-summary > \.metric-grid \{[^}]*repeat\(2, minmax\(0, 1fr\)\)/s,
  );
  assert.match(styles, /\.most-expensive-card \{[^}]*flex-direction: column/s);
  assert.match(styles, /\.most-expensive-visual \{[^}]*width: min\(100%, 250px\)/s);
  assert.doesNotMatch(overview, /id="metric-(?:coverage|unpriced)"/);
  assert.match(data, /id="metric-coverage"/);
  assert.match(data, /id="metric-unpriced"/);
  assert.doesNotMatch(html, />Carte la plus chère</);
  assert.doesNotMatch(html, /Prix unitaire de référence le plus élevé/);
});

test("keeps prices in the collection table and groups analysis rows by printing", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");

  assert.equal(
    (
      script.match(
        /appendCell\(row, formatCurrency\(item\.priceCents\), "price"\);/g,
      ) ?? []
    ).length,
    1,
  );
  assert.equal((html.match(/<th scope="col">Prix<\/th>/g) ?? []).length, 1);
  assert.doesNotMatch(html, /<th scope="col">Statut<\/th>/);
  assert.match(
    html,
    /data-sort-keys="name,setCode,collectorNumber,quantity,finish,rarity,colors,manaValue,priceCents"/,
  );
  assert.match(script, /const groupedItems = groupCardPrintings\(items\);/);
  assert.match(script, /formatCount\(groupedItems\.length\)/);
  assert.doesNotMatch(script, /item\.priceDate\s*\?\s*`/);
});

test("makes every card table sortable with accessible column controls", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const styles = readFileSync(new URL("./styles.css", import.meta.url), "utf8");
  const tables = [
    ...html.matchAll(/<table data-sort-keys="([^"]+)">([\s\S]*?)<\/table>/g),
  ];

  assert.equal(tables.length, 5);
  for (const [, keys, table] of tables) {
    assert.equal(keys.split(",").length, (table.match(/<th /g) ?? []).length);
    assert.equal(keys.split(",")[0], "name");
  }
  assert.match(script, /initializeSortableTables\(\);/);
  assert.match(script, /setAttribute\("aria-sort", state\.direction\)/);
  assert.match(styles, /\.sort-button/);
});

test("uses type-style metrics and two interactive charts on facet pages", () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");

  for (const kind of ["keywords", "sets", "rarities"]) {
    assert.match(html, new RegExp(`id="${kind}-metric-quantity"`));
    assert.match(html, new RegExp(`id="${kind}-metric-distinct-names"`));
    assert.match(html, new RegExp(`id="${kind}-metric-value"`));
    assert.match(html, new RegExp(`data-facet-mana-chart="${kind}"`));
    assert.match(html, new RegExp(`id="${kind}-analysis-rows"`));
    assert.match(html, new RegExp(`id="reset-${kind}-chart-filters"`));
  }
  assert.doesNotMatch(html, /Quantités par (mot-clé|extension|rareté)/);
  assert.doesNotMatch(html, /data-distribution="(keywords|sets|rarities)"/);
  assert.equal((html.match(/class="chart-grid chart-grid--type"/g) ?? []).length, 4);
});

test("updates analysis metrics from the cards selected by chart filters", () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");

  assert.match(
    script,
    /function renderFacetAnalysis[\s\S]*?const metrics = overviewMetrics\(filteredItems\)/,
  );
  assert.match(
    script,
    /function renderTypeAnalysisResults[\s\S]*?renderTypeMetrics\(items\)/,
  );
});

test("clears the Types counters when the last collection is deselected", async () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const fields = new Map(
    ["quantity", "distinct-names", "value"].map((key) => [
      `#type-metric-${key}`,
      { textContent: "42" },
    ]),
  );
  const context = {
    document: { querySelector: (id) => fields.get(id) },
    typeSelect: { value: "" },
    typeAnalysisPending: true,
    typeAnalysisRequest: 0,
    typeAnalysis: {},
    resetTypeChartFilters: {},
    typeAnalysisError: { hidden: false },
    typeAnalysisStatus: {},
    typeChartTargets: {},
    clearTypeChartFilters() {},
    renderChartEmpty() {},
    renderTypeItems() {},
    overviewMetrics,
    formatCount: String,
    formatCurrency: (cents) => `${cents} €`,
  };
  const analysis = script.slice(
    script.indexOf("function renderTypeMetrics("),
    script.indexOf("function renderDistributions("),
  );
  await runInNewContext(analysis + "\nloadTypeAnalysis();", context);
  assert.deepEqual([...fields.values()].map((field) => field.textContent), ["0", "0", "0 €"]);
  assert.equal(context.typeAnalysisError.hidden, true);
});

test("refreshes the complete snapshot even with a filter active on Cartes", async () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const context = {
    dashboardRequest: 0,
    collectionSnapshotCurrent: true,
    searchableCards: ["ancien"],
    typeAnalysisRequest: 0,
    statusRegion: {},
    window: { location: { hash: "#types" } },
    pageFromHash,
    rows: { replaceChildren() {} },
    virtualTables: new WeakMap(),
    clearError() {},
    setLoading() {},
    showError: assert.fail,
    currentQuery: () => ({ search: "Filtre" }),
    hasActiveQuery: (query) => Boolean(query.search),
    invoke: async (_, { query }) => ({
      items: query.search ? ["filtré"] : ["filtré", "nouveau"],
      distributions: { types: [] },
    }),
    renderMetrics() {},
    renderDistributions() {},
    renderOverviewAnalysis() {},
    renderTypeOptions() {},
    renderActiveCollectionPage() {},
    renderDiagnostics() {},
  };
  context.updateSearchableCards = (items) => {
    context.searchableCards = items;
    context.collectionSnapshotCurrent = true;
  };
  const load = script.slice(
    script.indexOf("async function loadDashboard("),
    script.indexOf('form.addEventListener("submit"'),
  );
  await runInNewContext(load + "\nloadDashboard(false);", context);
  assert.deepEqual(context.searchableCards, ["ancien"]);
  await runInNewContext("loadDashboard(true);", context);
  assert.deepEqual(context.searchableCards, ["filtré", "nouveau"]);
  assert.equal(context.collectionSnapshotCurrent, true);
});
