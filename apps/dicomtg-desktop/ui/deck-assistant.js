import { DECK_COLOR_CODES } from "./deck-builder.js";

export const DECK_FORMATS = {
  freeform: { label: "Libre", target: 60, lands: 24 },
  commander: { label: "Commander", target: 100, lands: 37 },
  standard: { label: "Standard", target: 60, lands: 24 },
  pioneer: { label: "Pioneer", target: 60, lands: 24 },
  modern: { label: "Modern", target: 60, lands: 24 },
  pauper: { label: "Pauper", target: 60, lands: 24 },
  legacy: { label: "Legacy", target: 60, lands: 24 },
  vintage: { label: "Vintage", target: 60, lands: 24 },
  limited: { label: "Limité", target: 40, lands: 17 },
};

const collator = new Intl.Collator("fr", { sensitivity: "base" });
const deckGroups = [
  ["creature", "Créatures"], ["planeswalker", "Planeswalkers"], ["instant", "Éphémères"],
  ["sorcery", "Rituels"], ["artifact", "Artefacts"], ["enchantment", "Enchantements"],
  ["battle", "Batailles"], ["land", "Terrains"], ["other", "Autres"],
];
const groupLabels = new Map([
  ["commander", "Commandants"], ...deckGroups, ["sideboard", "Sideboard"], ["considering", "Considering"],
]);
const typePriority = ["land", ...deckGroups.map(([name]) => name)];
const mechanics = [
  /\b(tokens?|populate)\b/i,
  /\b(artifact|artifacts|affinity|metalcraft)\b/i,
  /\b(graveyard|graveyards|delve|flashback|threshold)\b/i,
  /\b(counters?|proliferate)\b/i,
  /\b(sacrifice|sacrificed|dies)\b/i,
  /\b(landfall|lands?|ramp)\b/i,
  /\b(draw|draws|discard|discards)\b/i,
  /\b(gain life|gains? .* life|lifelink)\b/i,
  /\b(copy|copies|storm)\b/i,
];

export function deckCardKey(name) {
  return String(name ?? "").trim().normalize("NFC").toLocaleLowerCase("en");
}

export function isConsidering(board) {
  return ["considering", "maybeboard", "maybe board"].includes(board?.trim().toLowerCase());
}

export function resolveDeckPrinting(entry, metadata) {
  const explicit = entry.setCode && entry.collectorNumber;
  const samePrinting = !explicit || (entry.setCode === metadata?.setCode
    && entry.collectorNumber === metadata?.collectorNumber);
  return {
    ...metadata,
    name: metadata?.name || entry.name,
    typeLine: metadata?.typeLine || entry.cardType,
    manaCost: metadata?.manaCost || entry.manaCost,
    setCode: explicit ? entry.setCode : metadata?.setCode,
    collectorNumber: explicit ? entry.collectorNumber : metadata?.collectorNumber,
    scryfallId: samePrinting ? metadata?.scryfallId : null,
    catalogOnly: true,
  };
}

export function deckCoverCard(deck, info) {
  const commander = deck.cards.find((card) => card.board === "commander");
  if (commander) return commander;
  const main = deck.cards.filter((card) => card.board === "main");
  const rarity = { mythic: 4, rare: 3, uncommon: 2, common: 1 };
  return main.filter((card) => !isLand(info.get(deckCardKey(card.name)), card))
    .sort((a, b) => (rarity[info.get(deckCardKey(b.name))?.rarity] ?? 0)
      - (rarity[info.get(deckCardKey(a.name))?.rarity] ?? 0))[0] ?? main[0] ?? deck.cards[0];
}

export function associatedDeckTokens(cards, related, keyFor = deckCardKey) {
  const tokens = new Map();
  for (const card of cards) {
    if (isConsidering(card.board)) continue;
    for (const token of related.get(keyFor(card.name)) ?? []) tokens.set(token.scryfallId, token);
  }
  return [...tokens.values()];
}

export function groupDeckCards(cards, info) {
  const groups = new Map([...groupLabels].map(([key, label]) => [key, { label, quantity: 0, entries: [] }]));
  for (const [index, card] of cards.entries()) {
    let key = card.board;
    if (isConsidering(key)) key = "considering";
    if (key === "main") {
      const type = (info.get(deckCardKey(card.name))?.typeLine || card.cardType || "")
        .split(/[—–]|\/\//, 1)[0].toLowerCase().split(/\s+/);
      key = typePriority.find((name) => type.includes(name)) ?? "other";
    }
    if (!groups.has(key)) groups.set(key, { label: key, quantity: 0, entries: [] });
    const group = groups.get(key);
    group.entries.push({ card, index });
    group.quantity += card.quantity;
  }
  return [...groups.values()].filter((group) => group.entries.length).map((group) => ({
    ...group,
    entries: group.entries.sort((a, b) => collator.compare(a.card.name, b.card.name)),
  }));
}

export function moveDeckCard(cards, card, board) {
  if (card.board === board) return false;
  const sameCard = (entry) => deckCardKey(entry.name) === deckCardKey(card.name);
  if (board === "commander" && cards.some((entry) => entry.board === board && sameCard(entry))) return false;
  const count = board === "commander" ? 1 : card.quantity;
  const existing = cards.find((entry) => entry.board === board && sameCard(entry)
    && ["setCode", "collectorNumber", "printing", "language"].every((field) => (entry[field] ?? "") === (card[field] ?? "")));
  if (existing && existing.quantity + count > 4294967295) return false;
  const moved = { ...card, board, quantity: count };
  if (count === card.quantity) cards.splice(cards.indexOf(card), 1);
  else card.quantity -= count;
  if (existing) existing.quantity += count;
  else cards.push(moved);
  return true;
}

export function canBeCommander(info) {
  return /\bLegendary\b.*\bCreature\b/.test(info?.typeLine ?? "")
    || /\bcan be your commander\b/i.test(info?.rulesText ?? "");
}

export function deckColors(deck, info) {
  if (deck.format !== "commander") return deck.colors;
  const identity = new Set(deck.cards
    .filter((card) => card.board === "commander")
    .flatMap((card) => info.get(deckCardKey(card.name))?.colorIdentity ?? []));
  return DECK_COLOR_CODES.filter((color) => identity.has(color));
}

export function deckLibraryColors(deck, info) {
  const configured = deckColors(deck, info);
  if (deck.format === "commander" || configured.length) return configured;
  const colors = new Set(deck.cards.filter((card) => card.board === "main")
    .flatMap((card) => info.get(deckCardKey(card.name))?.colorIdentity ?? []));
  return DECK_COLOR_CODES.filter((color) => colors.has(color));
}

export function isLand(info, entry) {
  return /\bland\b/i.test((info?.typeLine ?? entry?.cardType ?? "").split(/[—–]|\/\//, 1)[0]);
}

function copyLimit(deck, info, status) {
  if (/\bBasic Land\b/i.test(info?.typeLine ?? "")) return Infinity;
  if (status === "restricted") return 1;
  if (deck.format === "limited" || deck.format === "freeform") return Infinity;
  const text = info?.rulesText ?? "";
  if (/a deck can have any number of cards named/i.test(text)) return Infinity;
  const limit = text.match(/a deck can have up to (\w+) cards? named/i)?.[1]?.toLowerCase();
  const exceptions = { four: 4, five: 5, seven: 7, nine: 9 };
  return exceptions[limit] ?? (deck.format === "commander" ? 1 : 4);
}

export function deckStats(deck, info) {
  const { target, lands: baseline } = DECK_FORMATS[deck.format] ?? DECK_FORMATS.freeform;
  let total = 0;
  let lands = 0;
  let manaTotal = 0;
  let manaSamples = 0;
  const curve = Array(8).fill(0);
  for (const entry of deck.cards) {
    if (entry.board !== "main" && entry.board !== "commander") continue;
    const card = info.get(deckCardKey(entry.name));
    total += entry.quantity;
    if (isLand(card, entry)) {
      lands += entry.quantity;
    } else if (Number.isFinite(card?.manaValue)) {
      const mana = Math.min(10, card.manaValue);
      manaTotal += mana * entry.quantity;
      manaSamples += entry.quantity;
      curve[Math.min(7, Math.floor(mana))] += entry.quantity;
    }
  }
  const correction = manaSamples < 5 ? 0 : Math.round((manaTotal / manaSamples - 3) * 2);
  const recommendedLands = baseline + Math.max(-3, Math.min(4, correction));
  return {
    target, total, lands, curve, recommendedLands,
    remaining: Math.max(0, target - total),
    spellSlots: Math.max(0, target - total - Math.max(0, recommendedLands - lands)),
  };
}

const basicLandColors = { plains: "W", island: "U", swamp: "B", mountain: "R", forest: "G" };
const basicLandName = /^(?:Snow-Covered )?(?:Plains|Island|Swamp|Mountain|Forest|Wastes)$/i;

function manaColors(text) {
  return [...text.matchAll(/\{([^}]+)\}/g)]
    .flatMap(([, symbol]) => symbol.split("/")).filter((color) => DECK_COLOR_CODES.includes(color));
}

export function deckLandInputs(deck, info, recommendedLands = 0) {
  const zeroes = () => Object.fromEntries(DECK_COLOR_CODES.map((color) => [color, 0]));
  const coloredCards = zeroes();
  const multicolorLands = zeroes();
  const landSources = [];
  let nonbasicLands = 0;
  for (const entry of deck?.cards ?? []) {
    if (entry.board !== "main" && entry.board !== "commander") continue;
    const card = info.get(deckCardKey(entry.name));
    const quantity = entry.quantity;
    if (!isLand(card, entry)) {
      for (const color of new Set(manaColors(card?.manaCost || entry.manaCost || ""))) {
        coloredCards[color] += quantity;
      }
      continue;
    }
    const [type, subtypes = ""] = (card?.typeLine || entry.cardType || "").split("//", 1)[0].split(/[—–]/, 2);
    if (/\bbasic\s+land\b/i.test(type) || basicLandName.test(entry.name)) continue;
    nonbasicLands += quantity;
    const producedMana = [...(card?.rulesText ?? "").matchAll(/\badd\b([^.!\n]*)/gi)]
      .map(([, effect]) => effect).join(" ");
    const colors = new Set(manaColors(producedMana));
    for (const subtype of subtypes.trim().toLowerCase().split(/\s+/)) {
      if (Object.hasOwn(basicLandColors, subtype)) colors.add(basicLandColors[subtype]);
    }
    const anyColor = /\b(?:mana of any (?:one )?color|mana in any combination of colors)\b/i.test(producedMana);
    landSources.push({ colors, anyColor, quantity });
  }
  const deckPalette = new Set(deck ? deckColors(deck, info) : []);
  for (const color of DECK_COLOR_CODES) if (coloredCards[color]) deckPalette.add(color);
  for (const { colors, anyColor, quantity } of landSources) {
    if (anyColor) for (const color of deckPalette) colors.add(color);
    if (colors.size < 2) continue;
    for (const color of colors) multicolorLands[color] += quantity;
  }
  return {
    basicLandSlots: Math.max(0, recommendedLands - nonbasicLands),
    multicolorLands,
    coloredCards,
  };
}

export function deckQuantities(cards) {
  const quantities = new Map();
  for (const card of cards) {
    if (isConsidering(card.board)) continue;
    const key = deckCardKey(card.name);
    quantities.set(key, (quantities.get(key) ?? 0) + card.quantity);
  }
  return quantities;
}

export function inventoryForDeck(workspace, currentDeckId, primaryId, keyFor = deckCardKey) {
  const names = new Map(workspace.collections.map(({ id, name }) => [id, name]));
  const byName = new Map();
  for (const item of workspace.holdings) {
    const key = keyFor(item.name);
    if (!byName.has(key)) byName.set(key, new Map());
    const collections = byName.get(key);
    collections.set(item.collectionId, (collections.get(item.collectionId) ?? 0) + item.quantity);
  }
  const availableHoldings = new Map();
  for (const [key, collections] of byName) {
    const holdings = [...collections].map(([collectionId, quantity]) => ({ collectionId, quantity }));
    availableHoldings.set(key, holdings.sort((a, b) => Number(b.collectionId === primaryId)
      - Number(a.collectionId === primaryId) || a.collectionId - b.collectionId));
  }
  const reservations = new Map();
  for (const deck of workspace.decks) {
    if (deck.id === currentDeckId) continue;
    const quantities = new Map();
    for (const card of deck.cards) {
      if (isConsidering(card.board)) continue;
      const key = keyFor(card.name);
      quantities.set(key, (quantities.get(key) ?? 0) + card.quantity);
    }
    for (const [key, quantity] of quantities) {
      if (!reservations.has(key)) reservations.set(key, []);
      reservations.get(key).push({ name: deck.name, quantity });
    }
  }
  return (name, quantity = 1, alreadyAssigned = 0) => {
    const key = keyFor(name);
    const holdings = availableHoldings.get(key) ?? [];
    const used = reservations.get(key) ?? [];
    const owned = holdings.reduce((total, item) => total + item.quantity, 0);
    const reserved = used.reduce((total, item) => total + item.quantity, 0);
    const available = Math.max(0, owned - reserved - alreadyAssigned);
    let skip = reserved + alreadyAssigned;
    let missing = quantity;
    const origins = [];
    for (const item of holdings) {
      const count = Math.min(missing, Math.max(0, item.quantity - skip));
      skip = Math.max(0, skip - item.quantity);
      if (count) origins.push({ collection: names.get(item.collectionId), quantity: count });
      missing -= count;
      if (!missing) break;
    }
    return {
      owned, available, missing, origins,
      holdings,
      conflicts: missing && owned ? [...new Set(used.map((deck) => deck.name))] : [],
    };
  };
}

function signals(info) {
  const tags = new Set((info.keywords ?? []).map((keyword) => keyword.toLowerCase()));
  const typeLine = info.typeLine ?? "";
  for (const type of typeLine.split(/[—–]/, 1)[0].match(/\b(Artifact|Creature|Enchantment|Instant|Sorcery|Planeswalker|Battle)\b/g) ?? []) {
    tags.add("type:" + type);
  }
  const subtype = info.typeLine?.split(/[—–]/, 2)[1] ?? "";
  for (const type of subtype.toLowerCase().match(/[\p{L}]+/gu) ?? []) tags.add(type);
  const text = info.rulesText ?? "";
  mechanics.forEach((pattern, index) => { if (pattern.test(text)) tags.add(index); });
  return tags;
}

export function recommendCards(deck, candidates, info, available, legalities = new Map()) {
  const stats = deckStats(deck, info);
  if (!stats.remaining) return [];
  const colors = new Set(deckColors(deck, info));
  const quantities = deckQuantities(deck.cards);
  const themes = new Map();
  for (const entry of deck.cards) {
    const card = info.get(deckCardKey(entry.name));
    if (isConsidering(entry.board) || entry.board === "sideboard" || !card || isLand(card)) continue;
    for (const tag of signals(card)) {
      themes.set(tag, (themes.get(tag) ?? 0) + (entry.board === "commander" ? 3 : 1));
    }
  }

  const ranked = [];
  for (const candidate of candidates) {
    const key = deckCardKey(candidate.name);
    const status = legalities.get(key)?.[deck.format];
    if (["banned", "not_legal", "suspended"].includes(status)) continue;
    const identity = deck.format === "commander" ? candidate.colorIdentity : candidate.colors ?? candidate.colorIdentity;
    if ((identity ?? []).some((color) => !colors.has(color))) continue;
    const copies = quantities.get(key) ?? 0;
    if (copies >= copyLimit(deck, candidate, status)) continue;
    const land = isLand(candidate);
    if (!land && !stats.spellSlots) continue;
    const stock = available(candidate.name, copies);
    if (!stock && !/\bBasic Land\b/i.test(candidate.typeLine ?? "")) continue;

    const mana = Math.min(7, Math.floor(candidate.manaValue ?? 0));
    const synergy = [...signals(candidate)].reduce((value, tag) => value + Math.min(3, themes.get(tag) ?? 0), 0);
    const score = land
      ? (stats.lands < stats.recommendedLands ? 7 : -8) - copies + (stock ? 2 : 0)
      : 3 * synergy + (mana <= 3 ? 5 : 2) - Math.min(5, stats.curve[mana]) - copies;
    ranked.push({ candidate, score });
  }
  return ranked.sort((a, b) => b.score - a.score || collator.compare(a.candidate.name, b.candidate.name))
    .slice(0, 16).map(({ candidate }) => candidate);
}

export function deckWarnings(deck, entry, metadata, info, legality, copies) {
  const warnings = [];
  const status = legality?.[deck.format];
  if (["banned", "not_legal", "suspended"].includes(status)) {
    warnings.push(status === "banned" ? "Bannie dans ce format" : "Non légale dans ce format");
  }
  if (copies > copyLimit(deck, metadata, status)) warnings.push("Limite d’exemplaires dépassée");
  const commanders = deck.cards.filter((card) => card.board === "commander");
  if (deck.format === "commander" && commanders.length && commanders.every((card) => info.has(deckCardKey(card.name)))) {
    if (entry.board === "commander" && metadata && !canBeCommander(metadata)) warnings.push("Commandant non admissible");
    if ((metadata?.colorIdentity ?? []).some((color) => !deckColors(deck, info).includes(color))) {
      warnings.push("Hors identité couleur");
    }
  }
  return { warnings, status };
}

export function deckLibraryStatus(deck, info, legalities, keyFor = deckCardKey) {
  const commanderFormat = deck.format === "commander";
  const { target } = DECK_FORMATS[deck.format];
  const commanders = deck.cards.filter((card) => card.board === "commander");
  const total = deck.cards.reduce((count, card) =>
    count + (card.board === "main" || (commanderFormat && card.board === "commander") ? card.quantity : 0), 0);
  if (total < target || (commanderFormat && !commanders.length)) return "incomplete";
  if (deck.format === "freeform") return "legal";
  if (deck.format === "limited") return "unverified";
  const sideboard = deck.cards.reduce((count, card) => count + (card.board === "sideboard" ? card.quantity : 0), 0);
  if (commanderFormat ? total > target || commanders.length > 2 || sideboard > 1 : commanders.length || sideboard > 15) {
    return "illegal";
  }

  const cards = deck.cards.filter((card) => !isConsidering(card.board));
  const quantities = new Map();
  for (const card of cards) {
    const key = keyFor(card.name);
    quantities.set(key, (quantities.get(key) ?? 0) + card.quantity);
  }
  let unverified = commanderFormat && (commanders.length === 2 || sideboard === 1);
  for (const card of cards) {
    const key = keyFor(card.name);
    const metadata = info.get(key);
    const { warnings, status } = deckWarnings(deck, card, metadata, info, legalities.get(key), quantities.get(key));
    if (["banned", "not_legal", "suspended"].includes(status) || (metadata && warnings.length)
      || (status === "restricted" && quantities.get(key) > 1)) return "illegal";
    if (!metadata || !["legal", "restricted"].includes(status)) unverified = true;
  }
  return unverified ? "unverified" : "legal";
}
