import assert from "node:assert/strict";
import test from "node:test";
import { CARD_LANGUAGES, setCardLanguage } from "./game-localization.js";
import {
  initializeInterfaceLanguage, refreshInterfaceLanguage, setUiAttribute, setUiText, t,
} from "./interface-language.js";
import { formatCount, formatImportSummary } from "./formatters.js";
import catalog from "./interface-messages.js";

function element(tagName, ...childNodes) {
  const attributes = new Map();
  const node = {
    nodeType: 1,
    tagName,
    childNodes,
    getAttribute: (attribute) => attributes.get(attribute) ?? null,
    setAttribute: (attribute, value) => attributes.set(attribute, value),
    hasAttribute: (attribute) => attributes.has(attribute),
    get textContent() { return this.childNodes.map((child) => child.nodeValue ?? child.textContent).join(""); },
    set textContent(value) { this.childNodes = [{ nodeType: 3, nodeValue: value, childNodes: [], parentNode: this }]; },
  };
  for (const child of childNodes) child.parentNode = node;
  return node;
}

test("every catalog entry covers all six target languages and keeps its interpolation parameters", () => {
  const placeholders = (string) => [...string.matchAll(/\{[a-zA-Z]+\}/g)].map(([token]) => token).sort();
  for (const [source, translations] of Object.entries(catalog)) {
    assert.equal(translations.length, Object.keys(CARD_LANGUAGES).length - 1, source);
    for (const translation of translations) {
      assert.ok(translation.trim(), source);
      assert.deepEqual(placeholders(translation), placeholders(source), source);
    }
  }
});

test("switches static text, templates, controls and active feedback in place", () => {
  const menu = element("BUTTON");
  menu.textContent = "Vue d’ensemble";
  menu.setAttribute("aria-label", "Vues de la collection");
  const template = element("TEMPLATE");
  template.content = element("FRAGMENT", element("H3"));
  template.content.childNodes[0].textContent = "Texte de la carte";
  const result = element("P");
  const title = element("H2");
  const root = element("HTML", menu, template, result, title);
  try {
    setCardLanguage("en");
    initializeInterfaceLanguage(root);
    setUiText(result, "Mettre à jour « {name} »", { name: "Cartes" });
    setUiAttribute(result, "title", "Actions pour {name}", { name: "Cartes" });
    title.textContent = "Cartes";
    assert.equal(menu.textContent, "Overview");
    assert.equal(result.textContent, "Update “Cartes”");

    setCardLanguage("ja");
    refreshInterfaceLanguage(root);
    assert.equal(root.lang, "ja");
    assert.equal(menu.textContent, "概要");
    assert.equal(menu.getAttribute("aria-label"), "コレクションの画面");
    assert.equal(template.content.textContent, "カードのテキスト");
    assert.equal(result.textContent, "「Cartes」を更新");
    assert.equal(result.getAttribute("title"), "Cartes の操作");
    assert.equal(title.textContent, "Cartes");

    setCardLanguage("fr");
    refreshInterfaceLanguage(root);
    assert.equal(menu.textContent, "Vue d’ensemble");
    assert.equal(result.textContent, "Mettre à jour « Cartes »");
  } finally {
    setCardLanguage("fr");
  }
});

test("updates numerical and import summaries for the selected language", () => {
  try {
    setCardLanguage("en");
    assert.equal(formatCount(1234), "1,234");
    assert.equal(t("Nouveau deck"), "New deck");
    assert.equal(formatImportSummary({ totalQuantity: 2, matchedPrintings: 1, invalidRows: 1 }),
      "2 cards imported · 1 printing matched · 1 invalid row");
    setCardLanguage("de");
    assert.equal(formatCount(1234), "1.234");
    assert.equal(t("Nouveau deck"), "Neues Deck");
  } finally {
    setCardLanguage("fr");
  }
});
