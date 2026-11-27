import assert from "node:assert/strict";
import test from "node:test";

import {
  associatedDeckTokens, deckColors, deckCoverCard, deckLandInputs, deckLibraryColors, deckLibraryStatus, deckQuantities, deckStats, deckWarnings, groupDeckCards, inventoryForDeck,
  moveDeckCard, recommendCards, resolveDeckPrinting,
} from "./deck-assistant.js";

test("uses the commander's identity, includes colorless cards and reserves land slots", () => {
  const guide = { name: "Guide", colorIdentity: ["G"], typeLine: "Legendary Creature", manaValue: 3 };
  const forest = { name: "Forest", colorIdentity: ["G"], typeLine: "Basic Land" };
  const relic = { name: "Relic", colorIdentity: [], typeLine: "Artifact", manaValue: 2 };
  const deluge = { name: "Deluge", colorIdentity: ["U"], typeLine: "Sorcery" };
  const pool = [guide, forest, relic, deluge];
  const info = new Map(pool.map((card) => [card.name.toLowerCase(), card]));
  const deck = { format: "commander", colors: ["U"], cards: [{ name: "Guide", quantity: 1, board: "commander" }] };
  assert.deepEqual(deckColors(deck, info), ["G"]);
  assert.equal(deckStats(deck, info).recommendedLands, 37);
  assert.deepEqual(new Set(recommendCards(deck, pool, info, () => 1).map(card => card.name)), new Set(["Relic", "Forest"]));
  deck.cards[0].quantity = 63;
  assert.deepEqual(recommendCards(deck, pool, info, () => 1).map(card => card.name), ["Forest"]);
});

test("prioritizes personal copies and reports the other decks only when copies run out", () => {
  const workspace = {
    collections: [{ id: 1, name: "Amie" }, { id: 2, name: "Ma collection" }],
    holdings: [
      { name: "Abrade", collectionId: 1, quantity: 1 },
      { name: "Abrade", collectionId: 2, quantity: 2 },
      { name: "Koth", collectionId: 2, quantity: 1 },
    ],
    decks: [
      { id: 1, name: "Autre deck", cards: [{ name: "Abrade", quantity: 1 }, { name: "Koth", quantity: 1 }] },
      { id: 2, name: "Deck édité", cards: [{ name: "Abrade", quantity: 1 }] },
    ],
  };
  const inventory = inventoryForDeck(workspace, 2, 2);
  assert.deepEqual(inventory("Abrade", 1).origins, [{ collection: "Ma collection", quantity: 1 }]);
  assert.deepEqual(inventory("Abrade", 1).conflicts, []);
  assert.deepEqual(inventory("Koth", 1).conflicts, ["Autre deck"]);
  assert.deepEqual(inventory("Abrade", 3).origins, [
    { collection: "Ma collection", quantity: 1 }, { collection: "Amie", quantity: 1 },
  ]);
});

test("favors shared mechanics once a deck has cards and reports bans without filtering the deck", () => {
  const card = (name, rulesText) => ({ name, rulesText, typeLine: "Creature", manaValue: 2, colors: ["R"] });
  const maker = card("Token Maker", "Create a token.");
  const match = card("Z Token Reward", "Your tokens have haste.");
  const unrelated = card("A Creature", "First strike");
  const info = new Map([maker, match, unrelated].map((entry) => [entry.name.toLowerCase(), entry]));
  const deck = { format: "standard", colors: ["R"], cards: [] };
  assert.equal(recommendCards(deck, [match, unrelated], info, () => 1)[0].name, unrelated.name);
  deck.cards.push({ name: maker.name, quantity: 1, board: "main" });
  assert.equal(recommendCards(deck, [match, unrelated], info, () => 1)[0].name, match.name);
  assert.deepEqual(deckWarnings(deck, deck.cards[0], maker, info, { standard: "banned" }, 1).warnings, [
    "Bannie dans ce format",
  ]);
  assert.equal(deck.cards[0].name, maker.name);
});

test("groups main-deck cards by the front type and keeps Considering separate", () => {
  const emergent = { name: "Emeritus of Truce // Swords to Plowshares", quantity: 2, board: "main" };
  const side = { name: "Abrade", quantity: 1, board: "sideboard", cardType: "instant" };
  const considering = { name: "Abrade", quantity: 1, board: "Maybe Board", cardType: "instant" };
  const land = { name: "Forest", quantity: 20, board: "main", cardType: "land" };
  const info = new Map([[emergent.name.toLowerCase(), { typeLine: "Creature — Cat Cleric // Instant" }]]);
  assert.deepEqual(groupDeckCards([emergent, side, considering, land], info).map(({ label, quantity }) => [label, quantity]), [
    ["Créatures", 2], ["Terrains", 20], ["Sideboard", 1], ["Considering", 1],
  ]);
  assert.equal(deckQuantities([side, considering]).get("abrade"), 1);
});

test("moves one copy to the command zone and merges identical printings in the sideboard", () => {
  const card = { name: "Guide", setCode: "TST", collectorNumber: "1", quantity: 3, board: "main" };
  const target = { ...card, quantity: 1, board: "sideboard" };
  const cards = [card, target];
  assert.equal(moveDeckCard(cards, card, "commander"), true);
  assert.equal(card.quantity, 2);
  assert.equal(cards[2].quantity, 1);
  assert.equal(moveDeckCard(cards, card, "commander"), false);
  assert.equal(moveDeckCard(cards, card, "sideboard"), true);
  assert.deepEqual(cards.map(({ board, quantity }) => [board, quantity]), [["sideboard", 3], ["commander", 1]]);
});

test("prefers the deck's exact printing for preview and legality", () => {
  const metadata = { name: "Abrade", setCode: "TST", collectorNumber: "2", scryfallId: "metadata-id" };
  const imported = resolveDeckPrinting({ name: "Abrade", setCode: "ABC", collectorNumber: "12" }, metadata);
  assert.equal(imported.setCode, "ABC");
  assert.equal(imported.collectorNumber, "12");
  assert.equal(imported.scryfallId, null);
  assert.equal(resolveDeckPrinting({ name: "Abrade" }, metadata).scryfallId, "metadata-id");
});

test("selects a deck cover and deduplicates playable token printings", () => {
  const forest = { name: "Forest", board: "main", cardType: "Land" };
  const maker = { name: "Maker // Other", board: "main", cardType: "Creature" };
  const commander = { name: "Leader", board: "commander" };
  const considering = { name: "Candidate", board: "considering" };
  const cards = [forest, maker, commander, considering];
  assert.equal(deckCoverCard({ cards }, new Map()), commander);
  assert.equal(deckCoverCard({ cards: [forest, maker] }, new Map()), maker);
  const info = new Map([["maker // other", { colorIdentity: ["U", "B"] }]]);
  assert.deepEqual(deckLibraryColors({ cards, format: "freeform", colors: [] }, info), ["U", "B"]);
  const clue = { name: "Clue", scryfallId: "clue-1" };
  const otherClue = { name: "Clue", scryfallId: "clue-2" };
  const related = new Map([
    ["maker // other", [clue]],
    ["leader", [clue, otherClue]],
    ["candidate", [{ name: "Food", scryfallId: "food" }]],
  ]);
  assert.deepEqual(associatedDeckTokens(cards, related), [clue, otherClue]);
});

test("counts front-face aliases and full names together in the same collection", () => {
  const inventory = inventoryForDeck({
    collections: [{ id: 1, name: "Ma collection" }],
    holdings: [
      { collectionId: 1, name: "Maker", quantity: 1 },
      { collectionId: 1, name: "Maker // Other", quantity: 2 },
    ],
    decks: [{ id: 2, name: "Autre deck", cards: [{ name: "Maker", quantity: 1 }] }],
  }, 1, 1, (name) => name.split(" // ")[0]);
  const result = inventory("Maker // Other", 2);
  assert.deepEqual(result.holdings, [{ collectionId: 1, quantity: 3 }]);
  assert.deepEqual(result.origins, [{ collection: "Ma collection", quantity: 2 }]);
});

test("prefills lands and colored costs from the main deck and command zone", () => {
  const info = new Map([
    ["leader", { typeLine: "Legendary Creature", manaCost: "{2}{U/R}{U}", colorIdentity: ["U", "R"] }],
    ["sorcery", { typeLine: "Sorcery", manaCost: "{R}{R}" }],
    ["dual", { typeLine: "Land — Island Mountain" }],
    ["tower", { typeLine: "Land", rulesText: "{T}: Add one mana of any color in your commander's color identity." }],
    ["utility", { typeLine: "Land", rulesText: "{T}: Add {C}." }],
    ["island", { typeLine: "Basic Land — Island" }],
  ]);
  const deck = { format: "commander", colors: [], cards: [
    { name: "Leader", quantity: 1, board: "commander" },
    { name: "Sorcery", quantity: 3, board: "main" },
    { name: "Dual", quantity: 1, board: "main" },
    { name: "Tower", quantity: 1, board: "main" },
    { name: "Utility", quantity: 1, board: "main" },
    { name: "Island", quantity: 8, board: "main" },
    { name: "Sorcery", quantity: 2, board: "sideboard" },
    { name: "Dual", quantity: 1, board: "considering" },
  ] };
  assert.deepEqual(deckLandInputs(deck, info, 37), {
    basicLandSlots: 34,
    multicolorLands: { W: 0, U: 2, B: 0, R: 2, G: 0 },
    coloredCards: { W: 0, U: 1, B: 0, R: 4, G: 0 },
  });
  assert.deepEqual(deckLandInputs(undefined, info), {
    basicLandSlots: 0,
    multicolorLands: { W: 0, U: 0, B: 0, R: 0, G: 0 },
    coloredCards: { W: 0, U: 0, B: 0, R: 0, G: 0 },
  });
});

test("classifies saved decks by size and verified legality", () => {
  const card = (name, quantity, board = "main") => ({ name, quantity, board });
  const forest = { typeLine: "Basic Land — Forest", colorIdentity: ["G"] };
  const leader = { typeLine: "Legendary Creature", colorIdentity: ["G"] };
  const info = new Map([["forest", forest], ["leader", leader], ["test spell", {}], ["side", {}]]);
  const legalities = new Map([
    ["forest", { standard: "legal", commander: "legal" }],
    ["leader", { commander: "legal" }],
    ["test spell", { standard: "banned" }],
    ["side", { standard: "legal" }],
  ]);
  const status = (format, ...cards) => deckLibraryStatus({ format, cards }, info, legalities);
  assert.equal(status("standard"), "incomplete");
  assert.equal(status("standard", card("Forest", 59), card("Forest", 1, "sideboard")), "incomplete");
  assert.equal(status("standard", card("Forest", 60), card("Test spell", 1, "considering")), "legal");
  assert.equal(status("standard", card("Forest", 60), card("Test spell", 1, "sideboard")), "illegal");
  assert.equal(status("standard", card("Forest", 60), card("Side", 5, "sideboard")), "illegal");
  assert.equal(status("standard", card("Forest", 59), card("Unknown", 1)), "unverified");
  assert.equal(status("standard", card("Unknown", 60)), "unverified");
  assert.equal(status("commander", card("Forest", 100)), "incomplete");
  assert.equal(status("commander", card("Forest", 99), card("Leader", 1, "commander")), "legal");
  assert.equal(status("commander", card("Forest", 100), card("Leader", 1, "commander")), "illegal");
  assert.equal(status("freeform", card("Unknown", 60)), "legal");
  assert.equal(status("limited", card("Forest", 40)), "unverified");
});
