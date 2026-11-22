import assert from "node:assert/strict";
import test from "node:test";

import {
  cardImageUrl,
  distributionLabel,
  finishLabel,
  formatCurrency,
  formatBytes,
  formatDiagnosticsSummary,
  formatImportSummary,
  formatList,
  formatOptionalText,
  formatPercent,
  formatReferenceUpdate,
  statusLabel,
} from "./formatters.js";

test("builds the Scryfall image URL for a matched printing", () => {
  assert.equal(
    cardImageUrl({
      uuid: "matched-uuid",
      scryfallId: "5FEDDBD0-3D29-4428-B70D-107C1E26930D",
      setCode: "WWK",
      collectorNumber: "1",
    }),
    "https://cards.scryfall.io/normal/front/5/f/5feddbd0-3d29-4428-b70d-107c1e26930d.jpg",
  );
  assert.equal(
    cardImageUrl(
      {
        uuid: "matched-uuid",
        scryfallId: "5FEDDBD0-3D29-4428-B70D-107C1E26930D",
        setCode: "WWK",
        collectorNumber: "1",
      },
      "small",
    ),
    "https://cards.scryfall.io/small/front/5/f/5feddbd0-3d29-4428-b70d-107c1e26930d.jpg",
  );
  assert.equal(
    cardImageUrl(
      {
        uuid: "matched-uuid",
        scryfallId: "5FEDDBD0-3D29-4428-B70D-107C1E26930D",
        setCode: "VOW",
        collectorNumber: "141",
      },
      "normal",
      "back",
    ),
    "https://cards.scryfall.io/normal/back/5/f/5feddbd0-3d29-4428-b70d-107c1e26930d.jpg",
  );
  assert.equal(
    cardImageUrl({
      uuid: "matched-uuid",
      setCode: "T ST",
      collectorNumber: "001/a",
    }),
    "https://api.scryfall.com/cards/t%20st/001%2Fa?format=image&version=normal&face=front",
  );
  assert.equal(
    cardImageUrl({ uuid: null, setCode: "TST", collectorNumber: "1" }),
    null,
  );
  assert.equal(
    cardImageUrl({
      uuid: "matched-uuid",
      scryfallId: "not-a-uuid",
      setCode: "TST",
      collectorNumber: "1",
    }),
    "https://api.scryfall.com/cards/tst/1?format=image&version=normal&face=front",
  );
});

test("formats missing detail values without implicit zero", () => {
  assert.equal(formatOptionalText(null), "Indisponible");
  assert.equal(formatOptionalText(""), "Indisponible");
  assert.equal(formatOptionalText(0), "0");
  assert.equal(formatList([]), "Indisponible");
  assert.equal(formatList(["W", "U"]), "W · U");
});

test("translates known distribution labels and preserves catalog values", () => {
  assert.equal(distributionLabel("colors", "U"), "Bleu");
  assert.equal(distributionLabel("colors", "C"), "Incolore");
  assert.equal(distributionLabel("rarities", "mythic"), "Mythique");
  assert.equal(distributionLabel("types", "Creature"), "Creature");
});

test("formats dashboard money and displays missing prices as zero", () => {
  assert.equal(formatCurrency(250), "2,50 €");
  assert.equal(formatCurrency(null), "0,00 €");
});

test("formats binary cache sizes for the data page", () => {
  assert.equal(formatBytes(0), "0 octet");
  assert.equal(formatBytes(1536), "1,5 Kio");
  assert.equal(formatBytes(250 * 1024 * 1024), "250 Mio");
});

test("formats coverage basis points and match statuses", () => {
  assert.equal(formatPercent(8750), "87,5 %");
  assert.equal(statusLabel("matched"), "Appariée");
  assert.equal(statusLabel("unmatched"), "Non appariée");
  assert.equal(statusLabel("ambiguous"), "Ambiguë");
});

test("labels the owned card finish", () => {
  assert.equal(finishLabel("normal"), "Normale");
  assert.equal(finishLabel("foil"), "Foil");
  assert.equal(finishLabel("etched"), "Foil gravée");
});

test("summarizes a collection import without hiding invalid rows", () => {
  assert.equal(
    formatImportSummary({
      validRows: 3,
      invalidRows: 1,
      matchedPrintings: 2,
      unmatchedRows: 1,
      ambiguousRows: 0,
      totalQuantity: 7,
    }),
    "7 cartes importées · 2 impressions appariées · 1 ligne non appariée · 1 ligne invalide",
  );
});

test("reports Sorted rows outside Magic folders separately", () => {
  assert.equal(
    formatImportSummary({ totalQuantity: 4, matchedPrintings: 1, ignoredRows: 2 }),
    "4 cartes importées · 1 impression appariée · 2 lignes hors des dossiers Magic ignorées",
  );
});

test("summarizes installed reference data", () => {
  assert.equal(
    formatReferenceUpdate({
      status: "updated",
      catalogCards: 1234,
      referencePrices: 987,
      updateDate: "2026-11-19",
    }),
    "1 234 cartes de catalogue · 987 prix de référence · actualisés le 2026-11-19",
  );
  assert.equal(
    formatReferenceUpdate({ status: "current", updateDate: "2026-11-19" }),
    "Données déjà actualisées le 2026-11-19.",
  );
});

test("summarizes persisted collection diagnostics", () => {
  assert.equal(
    formatDiagnosticsSummary({
      invalidRows: [{}, {}],
      unresolvedRows: [{}],
    }),
    "2 lignes invalides · 1 carte à vérifier",
  );
});
