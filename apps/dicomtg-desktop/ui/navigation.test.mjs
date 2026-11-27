import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { runInNewContext as runScript } from "node:vm";

import {
  filterOverviewItems, mostExpensiveOverviewItem, nextChartSelection,
  overviewDistribution, overviewDistributionFromTotals, overviewMetrics,
} from "./charts.js";
import { findCardsByName, groupCardPrintings, groupCardVariants } from "./card-search.js";
import { cardsTableCsv } from "./csv-export.js";
import { cardLanguage, displayedCardName, displayedSetName, setGameLocalizations } from "./game-localization.js";
import { countText, setUiAttribute, setUiText, t } from "./interface-language.js";
import { PAGE_IDS, pageFromHash } from "./navigation.js";

function runInNewContext(code, context) {
  return runScript(code, Object.assign(context, { countText, setUiAttribute, setUiText, t, cardLanguage }));
}

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

  assert.match(html, /class="card-detail-printings"/);
  assert.match(html, /class="card-detail-rules"/);
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

test("shows one Wurm detail with separate quantities, rules and galleries", () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const render = script.slice(
    script.indexOf("function renderCardDetailVariant("),
    script.indexOf("function renderMostExpensiveCard("),
  );
  const wurm = { name: "Wurm", rarity: "token", typeLine: "Token Creature — Wurm" };
  const large = { ...wurm, uuid: "large", power: "5", toughness: "5", quantity: 2 };
  const small = {
    ...wurm, uuid: "small", power: "3", toughness: "3", quantity: 1,
    keywords: ["Lifelink"], rulesText: "Lifelink",
  };
  const title = {};
  const displayed = [];
  const createBlock = () => {
    const fields = {
      ".card-variant-title": {},
      ".card-edhrec-gauges": { remove() {} },
      ".card-detail-grid": {},
      ".card-detail-rules p": {},
      ".card-detail-printings": [],
    };
    return { fields, querySelector: (selector) => fields[selector], setAttribute() {} };
  };
  runInNewContext(render + "\nshowCardDetail(small);", {
    small, findCardsByName, groupCardPrintings, groupCardVariants, displayedCardName,
    searchableCards: [large, small],
    cardDetailTitle: title, cardDetailSearch: {}, cardDetailSearchError: {},
    cardDetailContent: { append: (...blocks) => displayed.push(...blocks) },
    cardDetailTemplate: { content: {} }, cardDetailEmpty: {}, cardDetailRequestId: 0,
    document: { importNode: createBlock }, window: { requestAnimationFrame() {} },
    hideCardPreview() {}, clearCardDetail() {}, loadCardEdhrecStats() {},
    localizeCardVariants() {}, navigateToPage() {}, refreshCardDetailEdhrecStats: null,
    distributionLabel: (_subject, label) => label, formatCount: String,
    formatList: (values) => values.join(", "), formatOptionalText: (value) => value ?? "",
    createManaSymbolText() {}, createColorSymbols() {},
    appendDetailFact: (target, label, value) => { target[label] = value; },
    setManaSymbolText: (target, text) => { target.textContent = text; },
    renderCardPrinting: (item, gallery) => gallery.push(item.uuid),
  });
  assert.equal(title.textContent, "Wurm");
  assert.deepEqual(displayed.map(({ fields }) => [
    fields[".card-variant-title"].textContent,
    fields[".card-detail-grid"]["Quantité possédée"],
    fields[".card-detail-rules p"].textContent,
    fields[".card-detail-printings"],
  ]), [["Jeton 5/5", "2", "", ["large"]], ["Jeton 3/3 · Lifelink", "1", "Lifelink", ["small"]]]);
});

test("keeps the rules panel text-only and prefers French rules to received English text", async () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const rules = html.slice(
    html.indexOf('class="card-detail-rules"'),
    html.indexOf('class="card-detail-printings-section"'),
  );
  assert.match(rules, /<h3>Texte de la carte<\/h3>\s*<p><\/p>/);
  assert.doesNotMatch(rules, /<img\b|Texte français sur la carte imprimée/);

  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const localize = script.slice(
    script.indexOf("function loadAlternatePrintedText("),
    script.indexOf("function renderEdhrecGauge("),
  );
  for (const localizedText of [null, "Piochez une carte."]) {
    const printings = ["128", "335"].map((collectorNumber) => ({
      item: { uuid: "owned-printing", setCode: "EOE", collectorNumber },
      setName: {},
    }));
    const shownTexts = [];
    let finishSecondPrinting;
    let finishAlternateLookup;
    const localization = runInNewContext(localize + "\nlocalizeCardDetail(detail, 1);", {
      detail: { printings, rules: {} },
      cardDetailRequestId: 1,
      setManaSymbolText: (_node, text) => shownTexts.push(text),
      cardLanguage, displayedSetName,
      alternatePrintedTextRequests: new Map(),
      invoke: async (command, { collectorNumber, oracleId, language }) => {
        assert.equal(language, "fr");
        if (command === "load_card_printed_text") {
          assert.equal(oracleId, "card-id");
          return new Promise((resolve) => {
            finishAlternateLookup = () => resolve("Piochez deux cartes.");
          });
        }
        if (collectorNumber === "335") {
          return new Promise((resolve) => {
            finishSecondPrinting = () => resolve({ setName: "Edge of Eternities", localizedText });
          });
        }
        return { setName: "Edge of Eternities", fallbackText: "Draw a card.", oracleId: "card-id" };
      },
    });
    await new Promise(setImmediate);
    assert.deepEqual(shownTexts, ["Draw a card."]);
    if (localizedText) {
      finishSecondPrinting();
      await new Promise(setImmediate);
      finishAlternateLookup();
    } else {
      finishAlternateLookup();
      await new Promise(setImmediate);
      assert.deepEqual(shownTexts, ["Draw a card.", "Piochez deux cartes."]);
      finishSecondPrinting();
    }
    await localization;

    assert.deepEqual(shownTexts, ["Draw a card.", localizedText ?? "Piochez deux cartes."]);
  }
});

test("looks for other French printings only once per card across repeated visits", async () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const localize = script.slice(
    script.indexOf("function loadAlternatePrintedText("),
    script.indexOf("function renderEdhrecGauge("),
  );
  const printings = ["128", "335", "128s"].map((collectorNumber) => ({
    item: { setCode: "EOE", collectorNumber },
    setName: {},
  }));
  const searches = [];
  const texts = [];
  await runInNewContext(
    localize + "\n(async () => { await localizeCardDetail(detail, 1); await localizeCardDetail(detail, 1); })();",
    {
      detail: { printings, rules: {} },
      cardDetailRequestId: 1,
      cardLanguage, displayedSetName,
      alternatePrintedTextRequests: new Map(),
      setManaSymbolText: (_node, text) => texts.push(text),
      invoke: async (command, payload) => {
        if (command === "load_card_printed_text") {
          searches.push(payload.oracleId);
          return "Piochez une carte.";
        }
        return { setName: "Edge of Eternities", oracleId: "card-id" };
      },
    },
  );
  assert.deepEqual(searches, ["card-id"]);
  assert.deepEqual(texts, ["Piochez une carte.", "Piochez une carte."]);
});

test("renders the card's format legalities and ignores stale metadata", async () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const localize = script.slice(
    script.indexOf("function renderCardLegalities("),
    script.indexOf("function renderEdhrecGauge("),
  );
  const formats = [...html.matchAll(/<dd data-format="([^"]+)">/g)].map(([, format]) => ({
    dataset: { format },
  }));
  assert.deepEqual(formats.map(({ dataset }) => dataset.format), [
    "commander", "standard", "modern", "pauper", "legacy", "vintage", "pioneer",
  ]);
  assert.match(html, /<dt>Limité<\/dt>\s*<dd[^>]*>Selon l’événement<\/dd>/);

  const requests = [];
  const context = {
    cardLanguage, displayedSetName,
    cardDetailRequestId: 1,
    detail: {
      legalities: { querySelectorAll: () => formats },
      printings: [{ item: { setCode: "TST", collectorNumber: "1" }, setName: {} }],
    },
    invoke(command) {
      assert.equal(command, "load_card_localization");
      return new Promise((resolve) => requests.push(resolve));
    },
  };
  const first = runInNewContext(localize + "\nlocalizeCardDetail(detail, 1);", context);
  context.cardDetailRequestId = 2;
  const current = runInNewContext(
    "renderCardLegalities(detail.legalities, null); localizeCardDetail(detail, 2);",
    context,
  );
  assert.ok(formats.every(({ textContent }) => textContent === "Indisponible"));
  requests[1]({
    legalities: {
      commander: "legal", standard: "not_legal", modern: "banned",
      pauper: "not_legal", legacy: "legal", vintage: "restricted",
    },
  });
  await current;
  requests[0]({ legalities: { commander: "banned" } });
  await first;
  assert.deepEqual(formats.map(({ textContent }) => textContent), [
    "Légale", "Non légale", "Bannie", "Non légale", "Légale", "Restreinte", "Indisponible",
  ]);
});

test("displays distinct linked tokens below owned printings and loads visible thumbnails", async () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  assert.match(html, /Versions possédées[\s\S]*?<section[^>]+data-related-tokens hidden>/);
  const observerCode = script.slice(
    script.indexOf("const relatedTokenImages ="),
    script.indexOf("const PREVIEW_HOVER_DELAY_MS"),
  );
  const localize = script.slice(
    script.indexOf("function appendRelatedTokens("),
    script.indexOf("function renderEdhrecGauge("),
  );
  const createElement = () => ({
    children: [],
    append(...children) { this.children.push(...children); },
  });
  const gallery = createElement();
  const section = { hidden: true, querySelector: () => gallery };
  const first = { scryfallId: "deathtouch", name: "Wurm", typeLine: "Artifact Creature" };
  const second = { ...first, scryfallId: "lifelink" };
  const observed = [];
  const loaded = [];
  let notify;
  await runInNewContext(observerCode + localize + "\nlocalizeCardDetail(detail, 1);", {
    cardLanguage, displayedSetName, displayedCardName,
    document: { createElement },
    IntersectionObserver: class {
      constructor(callback) { notify = (entries) => callback(entries, this); }
      observe(frame) { observed.push(frame); }
      unobserve() {}
    },
    createCardImageFrame: () => ({ frame: { isConnected: true }, image: {}, imageState: {} }),
    showCardImage: (token, _image, _state, size) => loaded.push([token.scryfallId, size]),
    cardDetailRequestId: 1,
    detail: {
      relatedTokens: section,
      printings: ["223", "224"].map((collectorNumber) => ({
        item: { setCode: "SOM", collectorNumber }, setName: {},
      })),
    },
    invoke: async (_command, { collectorNumber }) => ({
      setName: "Scars of Mirrodin",
      relatedTokens: collectorNumber === "223" ? [first] : [first, second],
    }),
  });
  assert.equal(section.hidden, false);
  assert.deepEqual(gallery.children.map((figure) => figure.children[1].children[0].textContent), ["Wurm", "Wurm"]);
  assert.equal(observed.length, 2);
  notify([{ target: observed[0], isIntersecting: false }]);
  assert.deepEqual(loaded, []);
  observed[1].isConnected = false;
  notify(observed.map((target) => ({ target, isIntersecting: true })));
  assert.deepEqual(loaded, [["deathtouch", "small"]]);
});

test("shows French rules before loading English token links and reuses the lookup", async () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const code = script.slice(
    script.indexOf("function loadEnglishRelatedTokens("),
    script.indexOf("async function localizeCardVariants("),
  );
  const rules = {};
  const links = [];
  let lookups = 0;
  let finishLookup;
  const metadata = {
    setName: "Dominaria United", localizedText: "Créez un jeton Oiseau.",
    oracleId: "aether-channeler", needsEnglishTokenLookup: true,
  };
  const token = { scryfallId: "bird-printing", name: "Bird" };
  const context = {
    cardLanguage, displayedSetName,
    cardDetailRequestId: 1,
    detail: {
      printings: [{ item: { setCode: "DMU", collectorNumber: "42" }, setName: {} }],
      rules, relatedTokens: {},
    },
    englishRelatedTokenRequests: new Map(),
    invoke: async (command, payload) => {
      if (command === "load_card_localization") return metadata;
      assert.equal(command, "load_card_related_tokens");
      assert.equal(payload.oracleId, "aether-channeler");
      assert.equal(payload.setCode, "DMU");
      assert.equal(payload.collectorNumber, "42");
      lookups++;
      return new Promise((resolve) => { finishLookup = () => resolve([token]); });
    },
    appendRelatedTokens: (_section, tokens) => links.push(...tokens),
    setManaSymbolText: (target, text) => { target.text = text; },
  };
  const first = runInNewContext(code + "\nlocalizeCardDetail(detail, 1);", context);
  await new Promise(setImmediate);
  assert.equal(rules.text, "Créez un jeton Oiseau.");
  assert.equal(lookups, 1);
  assert.deepEqual(links, []);
  context.cardDetailRequestId = 2;
  finishLookup();
  await first;
  assert.deepEqual(links, []);

  await runInNewContext("localizeCardDetail(detail, 2);", context);
  assert.equal(lookups, 1);
  assert.deepEqual(links, [token]);
});

test("displays the EDHREC gauges with independent values and the actual rank", async () => {
  const html = readFileSync(new URL("./index.html", import.meta.url), "utf8");
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const gauges = script.slice(
    script.indexOf("function renderEdhrecGauge("),
    script.indexOf("function renderCardDetailVariant("),
  );
  assert.match(html, /data-salt-gauge[\s\S]*?pathLength="1"/);
  assert.match(html, /data-rank-gauge[\s\S]*?pathLength="1"/);
  assert.match(html, /<h3>Ratio de classement<\/h3>/);

  function gauge() {
    const value = {};
    const fill = {};
    return {
      value,
      fill,
      querySelector(selector) {
        return selector === "[data-gauge-fill]"
          ? { setAttribute: (name, text) => { fill[name] = text; } }
          : value;
      },
      toggleAttribute(_name, empty) { this.empty = empty; },
    };
  }

  for (const [stats, saltText, rankText] of [
    [{ saltScore: 2.38, rank: 188, rankRatio: 0.813 }, "2,38", "0,813"],
    [{ saltScore: 0, rank: null, rankRatio: null }, "0", "Indisponible"],
    [{ saltScore: null, rank: 1001, rankRatio: 0 }, "Indisponible", "0,000"],
    [{ saltScore: 4, rank: 1, rankRatio: 1 }, "4", "1,000"],
  ]) {
    const cardEdhrecSalt = gauge();
    const cardEdhrecRank = gauge();
    const cardEdhrecRankPosition = {};
    const invoke = async (command, { uuid }) => {
      assert.equal(command, "load_card_edhrec_stats");
      assert.equal(uuid, "printing-id");
      return stats;
    };
    const nodes = {
      "[data-salt-gauge]": cardEdhrecSalt,
      "[data-rank-gauge]": cardEdhrecRank,
      "[data-rank-position]": cardEdhrecRankPosition,
    };
    await runInNewContext(gauges + "\nloadCardEdhrecStats(detail, 1);", {
      cardDetailRequestId: 1,
      detail: { uuid: "printing-id", edhrecLookupId: 0, block: { querySelector: (selector) => nodes[selector] } },
      formatCount: String,
      invoke,
    });
    assert.equal(cardEdhrecSalt.value.textContent, saltText);
    assert.equal(cardEdhrecSalt.empty, stats.saltScore == null);
    assert.equal(cardEdhrecSalt.fill["stroke-dasharray"], `${(stats.saltScore ?? 0) / 4} 1`);
    assert.equal(cardEdhrecRank.value.textContent, rankText);
    assert.equal(cardEdhrecRank.empty, stats.rankRatio == null);
    assert.equal(cardEdhrecRank.fill["stroke-dasharray"], `${stats.rankRatio ?? 0} 1`);
    assert.equal(cardEdhrecRankPosition.textContent, stats.rank == null ? "" : `n° ${stats.rank}`);
    if (stats.saltScore === 0) assert.match(cardEdhrecSalt.fill.stroke, /^hsl\(120,/);
    if (stats.saltScore === 4) assert.match(cardEdhrecSalt.fill.stroke, /^hsl\(0,/);
    if (stats.rankRatio === 0) assert.match(cardEdhrecRank.fill.stroke, /^hsl\(0,/);
    if (stats.rankRatio === 1) assert.match(cardEdhrecRank.fill.stroke, /^hsl\(120,/);
  }
});

test("refreshes the open card when the catalogue arrives even if the price update fails", async () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const refresh = script.slice(
    script.indexOf("async function refreshReferenceData("),
    script.indexOf('referenceButton.addEventListener("click"'),
  );
  const events = [];
  let catalogReady;
  let rejectPrices;
  const update = runInNewContext(refresh + "\nrefreshReferenceData(false);", {
    appUpdater: {},
    referenceButton: {},
    clearReferenceError() {},
    referenceResult: {},
    setReferenceLoading() {},
    statusRegion: {},
    todayLocalIso: () => "2026-11-26",
    tauriCore: { Channel: class {} },
    invoke(command, payload) {
      assert.equal(command, "update_reference_data");
      catalogReady = payload.catalogReady;
      return new Promise((_resolve, reject) => { rejectPrices = reject; });
    },
    refreshCardDetailEdhrecStats() { events.push("card refreshed"); },
    loadDashboard() {},
    showReferenceError() { events.push("price update failed"); },
  });

  catalogReady.onmessage(null);
  assert.deepEqual(events, ["card refreshed"]);
  rejectPrices(new Error("Prix indisponibles"));
  await update;
  assert.deepEqual(events, ["card refreshed", "price update failed"]);
});

test("a late EDHREC response cannot clear an updated card", async () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const lookup = script.slice(
    script.indexOf("async function loadCardEdhrecStats("),
    script.indexOf("function renderCardDetailVariant("),
  );
  const requests = [];
  const rendered = [];
  const completed = runInNewContext(
    lookup + "\nPromise.all([loadCardEdhrecStats(detail, 1), loadCardEdhrecStats(detail, 1)]);",
    {
      cardDetailRequestId: 1,
      detail: { uuid: "id", edhrecLookupId: 0 },
      invoke: () => new Promise((resolve) => requests.push(resolve)),
      renderCardEdhrecStats: (stats) => rendered.push(stats.rank),
    },
  );

  requests[1]({ rank: 5679 });
  requests[0]({ rank: null });
  await completed;
  assert.deepEqual(rendered, [5679]);
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
    cardFilterOptionsDirty: false,
    refreshNameSuggestions() {},
    refreshDetailSuggestions() {},
    refreshCardFilterOptions() {},
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

test("renders Overview on demand and reuses totals for the chart's own filter", () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const charts = new Map();
  const items = [
    { name: "Land", status: "matched", colors: [], types: ["Land"], manaValue: 0, quantity: 3, priceCents: 20 },
    { name: "Creature", status: "matched", colors: ["B"], types: ["Creature", "Kindred"], manaValue: 2, quantity: 2, priceCents: 100 },
    { name: "Spell", status: "matched", colors: ["R"], types: ["Instant"], manaValue: 1, quantity: 1, priceCents: 401 },
  ];
  const totals = {
    colors: [{ label: "C", quantity: 3 }, { label: "B", quantity: 2 }, { label: "R", quantity: 1 }],
    types: [{ label: "Land", quantity: 3 }, { label: "Creature", quantity: 2 }, { label: "Kindred", quantity: 2 }, { label: "Instant", quantity: 1 }],
    manaValues: [{ label: "-0", quantity: 3 }, { label: "2", quantity: 2 }, { label: "1", quantity: 1 }],
  };
  const context = {
    facetAnalyses: {},
    overviewDashboard: { dirty: true, metrics: overviewMetrics(items), distributions: totals },
    overviewChartFilters: { colors: [], types: [], manaValues: [] },
    searchableCards: items,
    chartTargets: { colors: [{}], types: [{}], manaValues: [{}] },
    resetOverviewChartFilters: {},
    overviewFilterSummary: {},
    filterOverviewItems, mostExpensiveOverviewItem, nextChartSelection,
    overviewDistribution, overviewDistributionFromTotals, overviewMetrics,
    renderOverviewMetrics: (metrics) => { context.quantity = metrics.totalQuantity; },
    renderMostExpensiveCard() {},
    renderDonutChart: (kind, entries, _target, options) => charts.set(kind, { entries, options }),
    renderManaChart: (entries, _target, options) => charts.set("manaValues", { entries, options }),
    focusChartValue() {},
    formatCount: String,
  };
  const route = script.slice(script.indexOf("function renderActiveCollectionPage("), script.indexOf("function navigateToPage("));
  const overview = script.slice(script.indexOf("function renderOverviewAnalysis("), script.indexOf("function renderTypeOptions("));
  runInNewContext(route + overview + '\nrenderActiveCollectionPage("data");', context);
  assert.equal(charts.size, 0);
  runInNewContext('renderActiveCollectionPage("overview");', context);
  assert.equal(context.quantity, 6);
  assert.deepEqual(charts.get("types").entries, overviewDistribution(items, "types"));
  assert.equal(charts.get("manaValues").entries[0].label, "0");
  const originalColors = charts.get("colors");
  runInNewContext('renderActiveCollectionPage("overview");', context);
  assert.equal(charts.get("colors"), originalColors);
  charts.get("manaValues").options.onSelect("2", false);
  assert.equal(context.quantity, 2);
  assert.deepEqual(charts.get("colors").entries, [{ label: "B", quantity: 2 }]);
  assert.equal(charts.get("manaValues").entries.length, 3);
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

test("refreshes the complete snapshot and reapplies the filter locally on Cartes", async () => {
  const script = readFileSync(new URL("./app.js", import.meta.url), "utf8");
  const context = {
    setGameLocalizations,
    openedCardDetail: null,
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
    invoke: async (_, { query }) => {
      assert.equal(query.search, "");
      assert.equal(query.colors.length, 0);
      return { items: ["filtré", "nouveau"], distributions: { types: [] } };
    },
    renderCollectionFilterResults() { context.collectionItems = context.searchableCards.filter((item) => item === "filtré"); },
    renderMetrics() {},
    renderDistributions() {},
    renderOverviewAnalysis() {},
    renderTypeOptions() {},
    renderActiveCollectionPage() {},
    deckWorkspace: { invalidate() {} },
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
  assert.deepEqual(context.searchableCards, ["filtré", "nouveau"]);
  assert.deepEqual(context.collectionItems, ["filtré"]);
  await runInNewContext("loadDashboard(true);", context);
  assert.deepEqual(context.searchableCards, ["filtré", "nouveau"]);
  assert.deepEqual(context.collectionItems, ["filtré"]);
  assert.equal(context.collectionSnapshotCurrent, true);
});
