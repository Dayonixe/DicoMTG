import assert from "node:assert/strict";
import test from "node:test";

import { manaSymbolParts, manaSymbolUrl } from "./mana-symbols.js";

test("builds official Scryfall SVG URLs for simple and compound mana symbols", () => {
  assert.equal(manaSymbolUrl("{W}"), "https://svgs.scryfall.io/card-symbols/W.svg");
  assert.equal(
    manaSymbolUrl("{W/U}"),
    "https://svgs.scryfall.io/card-symbols/WU.svg",
  );
  assert.equal(
    manaSymbolUrl("{2/W}"),
    "https://svgs.scryfall.io/card-symbols/2W.svg",
  );
  assert.equal(manaSymbolUrl("W"), null);
  assert.equal(manaSymbolUrl("{not mana!}"), null);
});

test("preserves surrounding card text while extracting mana symbols", () => {
  assert.deepEqual(manaSymbolParts("Payez {2}{R}."), [
    "Payez ",
    {
      symbol: "{2}",
      label: "Deux manas génériques",
      url: "https://svgs.scryfall.io/card-symbols/2.svg",
    },
    {
      symbol: "{R}",
      label: "Mana rouge",
      url: "https://svgs.scryfall.io/card-symbols/R.svg",
    },
    ".",
  ]);
});
