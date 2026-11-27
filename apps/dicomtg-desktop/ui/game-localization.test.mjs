import assert from "node:assert/strict";
import test from "node:test";
import { filterCollectionCards } from "./card-filters.js";
import { cardNameSuggestions, findCardsByName } from "./card-search.js";
import { cardsTableCsv } from "./csv-export.js";
import { distributionLabel } from "./formatters.js";
import { displayedCardName, setCardLanguage, setGameLocalizations } from "./game-localization.js";
import { sortCards } from "./sorting.js";

test("language changes the displayed labels, while searches accept every available name", () => {
  const abrade = { name: "Abrade", setCode: "AKH" };
  const birds = { name: "Birds of Paradise", setCode: "RAV" };
  const secondPrinting = { ...birds, setCode: "DMR" };
  const cards = [abrade, birds, secondPrinting];
  setGameLocalizations({
    Abrade: { fr: ["Fissuration"], de: ["Abrieb"] },
    "Birds of Paradise": { fr: ["Oiseaux de paradis"] },
  }, { RAV: { en: "Ravnica: City of Guilds", fr: "Ravnica : la Cité des guildes" } });
  try {
    setCardLanguage("fr");
    assert.deepEqual(cardNameSuggestions(cards, "birds"), ["Oiseaux de paradis"]);
    assert.deepEqual(findCardsByName(cards, "oiseaux de paradis"), [birds, secondPrinting]);
    assert.deepEqual(filterCollectionCards(cards, { name: "abrieb" }), [abrade]);
    assert.equal(distributionLabel("keywords", "Flying"), "Vol");
    assert.equal(distributionLabel("sets", "RAV"), "Ravnica : la Cité des guildes");
    assert.equal(distributionLabel("sets", "MH3"), "Horizons du Modern 3");
    assert.equal(distributionLabel("keywords", "Unlisted"), "Unlisted");
    assert.match(cardsTableCsv([birds], ["name", "setCode"]), /Oiseaux de paradis;Ravnica : la Cité des guildes/);

    setCardLanguage("de");
    assert.equal(displayedCardName(abrade), "Abrieb");
    assert.equal(displayedCardName(birds), "Birds of Paradise");
    assert.deepEqual(cardNameSuggestions(cards, "Fissuration"), ["Abrieb"]);
    assert.deepEqual(sortCards(cards), cards);
  } finally {
    setGameLocalizations({}, {});
    setCardLanguage("fr");
  }
});

test("a French query can omit accents and resolves to the canonical group", () => {
  const cards = [
    { name: "Café // Home", localizedNames: { fr: ["Café // À la maison", "À la maison"] } },
    { name: "Café // Home" },
  ];
  assert.deepEqual(findCardsByName(cards, "a la maison"), cards);
});
