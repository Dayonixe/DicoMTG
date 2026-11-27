import { DECK_COLOR_CODES } from "./deck-builder.js";
import {
  DECK_FORMATS, associatedDeckTokens, canBeCommander, deckCardKey, deckColors, deckCoverCard, deckLibraryColors, deckLibraryStatus, deckQuantities, deckStats,
  deckLandInputs, deckWarnings, groupDeckCards, inventoryForDeck, isConsidering, moveDeckCard,
  recommendCards, resolveDeckPrinting,
} from "./deck-assistant.js";
import { compareLocalized, displayedCardName } from "./game-localization.js";
import { setUiText, t } from "./interface-language.js";

const BOARDS = { main: "Deck principal", commander: "Commandant", sideboard: "Sideboard", considering: "Considering" };
const collator = new Intl.Collator("fr", { sensitivity: "base" });
const emptyWorkspace = { collections: [], holdings: [], candidates: [] };
const noCommander = "Aucun commandant choisi.";
const deckBadgeLabels = {
  incomplete: ["–", "Deck incomplet", "illegal"],
  illegal: ["!", "Deck complet · légalité non respectée", "warning"],
  unverified: ["!", "Deck complet · légalité à vérifier", "warning"],
  legal: ["✓", "Deck complet et légal", "legal"],
};

function element(tag, text, className) {
  const node = document.createElement(tag);
  if (text != null) node.textContent = text;
  if (className) node.className = className;
  return node;
}

function option(value, name) {
  const entry = element("option");
  setUiText(entry, name);
  entry.value = value;
  return entry;
}

function action(label, handler, accessibleLabel) {
  const button = element("button", label);
  button.type = "button";
  if (accessibleLabel) button.setAttribute("aria-label", accessibleLabel);
  button.addEventListener("click", handler);
  return button;
}

function note(message) {
  return element("p", t(message), "analytics-note");
}

export function createDeckWorkspace(invoke, {
  createColorSymbols, formatCount, showCardDetail, showCardImage, clearCardImage, prefillLandBuilder,
}) {
  const root = document.querySelector("#deck");
  const get = (id) => root.querySelector("#" + id);
  const controls = Object.fromEntries([
    "saved-decks", "deck-library", "deck-library-actions", "deck-status", "deck-error", "deck-empty",
    "deck-editor", "deck-name", "deck-format", "deck-colors", "deck-progress",
    "deck-commanders", "deck-card-list", "deck-card-search", "deck-search-label",
    "deck-add-board", "deck-search-results", "deck-recommendations", "deck-import-file",
    "deck-import-dialog", "deck-import-choice", "deck-load-state", "deck-card-menu",
    "deck-quantity-dialog", "deck-quantity-add", "deck-preview-image", "deck-preview-state",
    "deck-preview-name",
    "deck-associated-tokens", "deck-token-list", "deck-collection-tooltip",
  ].map((id) => [id, get(id)]));
  const savedList = controls["saved-decks"];
  const searchInput = controls["deck-card-search"];
  const resultList = controls["deck-search-results"];
  const boardInput = controls["deck-add-board"];
  const importDialog = controls["deck-import-dialog"];
  const importChoice = controls["deck-import-choice"];
  const fileInput = controls["deck-import-file"];
  const status = controls["deck-status"];
  const error = controls["deck-error"];
  const cardMenu = controls["deck-card-menu"];
  const quantityDialog = controls["deck-quantity-dialog"];
  const previewImage = controls["deck-preview-image"];
  const previewState = controls["deck-preview-state"];
  const previewName = controls["deck-preview-name"];
  const tooltip = controls["deck-collection-tooltip"];
  const info = new Map();
  const legalities = new Map();
  const relatedTokens = new Map();
  const checked = new Set();
  const libraryBadges = new Map();
  const inputs = new Map();
  let workspace = emptyWorkspace;
  let savedDecks = [];
  let libraryReady = false;
  let libraryDirty = true;
  let libraryReading;
  let libraryReadingCards;
  let libraryToCheck = [];
  let draft;
  let stale = true;
  let loading;
  let workspaceReading = false;
  let cardReading;
  let busy = false;
  let dirty = false;
  let pendingImport;
  let replaceImport;
  let primaryId = Number(localStorage.getItem("dicomtg.deck.primaryCollection") ?? 0);
  let searchRevision = 0;
  let searchTimer;
  let legalityTimer;
  let checkingLegalities = false;
  let menuTrigger;
  let quantityTarget;
  let previewTimer;
  let previewKey;
  let tokenSignature = "";
  let languageDirty = false;
  let landInputsSignature;
  const imageRequests = new WeakMap();
  const imageObserver = new IntersectionObserver((entries, observer) => {
    for (const { target, isIntersecting } of entries) {
      if (!isIntersecting) continue;
      observer.unobserve(target);
      const request = imageRequests.get(target);
      imageRequests.delete(target);
      if (request && target.isConnected) void showCardImage(request.card, request.image, request.state, "small");
    }
  }, { rootMargin: "90px" });

  for (const [key, format] of Object.entries(DECK_FORMATS)) {
    controls["deck-format"].append(option(key, format.label));
  }
  const colorOptions = element("div", null, "color-options");
  for (const color of DECK_COLOR_CODES) {
    const input = element("input");
    input.type = "checkbox";
    input.value = color;
    const label = element("label");
    label.append(input, createColorSymbols([color]));
    inputs.set(color, input);
    input.addEventListener("change", () => {
      draft.colors = DECK_COLOR_CODES.filter((value) => inputs.get(value).checked);
      changed();
    });
    colorOptions.append(label);
  }
  controls["deck-colors"].append(colorOptions);

  function showError(message) {
    setUiText(error, message ? String(message) : "");
    error.hidden = !message;
  }

  function remember(cards) {
    for (const card of cards) {
      info.set(deckCardKey(card.name), card);
      if (card.requestedName) info.set(deckCardKey(card.requestedName), card);
    }
  }

  const keyFor = (name) => deckCardKey(info.get(deckCardKey(name))?.name ?? name);
  const cardLabel = (card) => displayedCardName(info.get(deckCardKey(card.name)) ?? card);

  function draftInventory() {
    return inventoryForDeck({ ...workspace, decks: savedDecks }, draft.id, primaryId, keyFor);
  }

  function clearImages(container) {
    for (const image of container.querySelectorAll(".card-image-frame img")) {
      const frame = image.parentElement;
      imageObserver.unobserve(frame);
      imageRequests.delete(frame);
      clearCardImage(image, frame.querySelector(".deck-image-status"));
    }
  }

  function imageFrame(label, frame = element("div")) {
    frame.classList.add("card-image-frame");
    const image = element("img", null, "card-detail-image");
    image.hidden = true;
    image.decoding = "async";
    image.referrerPolicy = "no-referrer";
    const state = element("span", label, "deck-image-status");
    frame.append(image, state);
    return { frame, image, state };
  }

  function queueImage(visual, card) {
    if (!card?.scryfallId && !(card?.setCode && card?.collectorNumber)) return;
    imageRequests.set(visual.frame, { ...visual, card });
    imageObserver.observe(visual.frame);
  }

  function refreshLibraryBadges() {
    libraryToCheck = [];
    for (const [deck, badge] of libraryBadges) {
      const state = deckLibraryStatus(deck, info, legalities, keyFor);
      if (state !== badge.dataset.deckState) {
        badge.replaceChildren(dot(...deckBadgeLabels[state]));
        badge.dataset.deckState = state;
      }
      if (state === "unverified" && deck.format !== "limited") libraryToCheck.push(deck);
    }
  }

  function renderLibrary() {
    controls["deck-library"].hidden = !!draft;
    controls["deck-library-actions"].hidden = !!draft;
    if (draft || !libraryDirty) return;
    libraryDirty = false;
    closeActionMenu();
    const decks = [...savedDecks].sort((a, b) => collator.compare(a.name, b.name) || a.id - b.id);
    controls["deck-empty"].hidden = !libraryReady || decks.length > 0;
    clearImages(savedList);
    savedList.replaceChildren();
    libraryBadges.clear();
    const covers = [];
    for (const deck of decks) {
      const row = element("li", null, "deck-panel deck-library-card");
      const edit = () => {
        if (!mayDiscard()) return;
        openDeck(deck);
        controls["deck-name"].focus();
      };
      const coverButton = action("", edit, t("Ouvrir le deck {name}", { name: deck.name }));
      coverButton.className = "deck-library-cover";
      coverButton.tabIndex = -1;
      const cover = deckCoverCard(deck, info);
      const visual = imageFrame(cover?.name ?? t("Deck vide"), coverButton);
      const body = element("div");
      const heading = element("div", null, "deck-library-heading");
      const title = element("h3");
      const openButton = action(deck.name, edit);
      openButton.className = "deck-library-title";
      title.append(openButton);
      const label = t("Actions pour {name}", { name: deck.name });
      heading.append(title, menuButton(label, (trigger) => {
        openActionMenu(trigger, label, [
          ["Modifier", edit],
          ["Mettre à jour via import", () => selectImport(deck)],
          ["Dupliquer", () => duplicateDeck(deck)],
          ["Supprimer", () => removeDeck(deck)],
        ]);
      }));
      const format = element("p", DECK_FORMATS[deck.format].label, "analytics-note deck-library-format");
      const badge = element("span", null, "deck-library-badge");
      format.append(badge);
      libraryBadges.set(deck, badge);
      body.append(heading, format);
      const colorSlot = element("div", null, "deck-library-colors");
      const colors = deckLibraryColors(deck, info);
      colorSlot.append(colors.length ? createColorSymbols(colors) : note("Couleurs à définir"));
      body.append(colorSlot);
      row.append(visual.frame, body);
      savedList.append(row);
      if (cover) covers.push({ cover, visual, deck, colorSlot });
    }
    const updateCovers = () => {
      for (const { visual, deck, colorSlot } of covers) {
        if (!visual.frame.isConnected) continue;
        const cover = deckCoverCard(deck, info);
        visual.state.textContent = cover.name;
        queueImage(visual, resolveDeckPrinting(cover, info.get(deckCardKey(cover.name))));
        const colors = deckLibraryColors(deck, info);
        if (colors.length) colorSlot.replaceChildren(createColorSymbols(colors));
        else if (deck.format === "commander" && deck.cards.some((card) =>
          card.board === "commander" && info.has(deckCardKey(card.name)))) {
          colorSlot.replaceChildren(note("Incolore"));
        }
      }
    };
    refreshLibraryBadges();
    const coverNames = covers.flatMap(({ cover, deck }) => {
        const colorBoard = deck.format === "commander" ? "commander" : deck.colors.length ? null : "main";
        return [cover, ...deck.cards.filter((card) => card.board === colorBoard)];
      });
    const names = [...new Set([...coverNames, ...libraryToCheck.flatMap(({ cards }) =>
      cards.filter((card) => !isConsidering(card.board)))]
      .filter((card) => !info.has(deckCardKey(card.name))).map((card) => card.name))];
    const finish = () => {
      updateCovers();
      refreshLibraryBadges();
      scheduleLegalities();
    };
    if (invoke && names.length) {
      const reading = invoke("search_deck_cards", { search: "", names, commander: false });
      libraryReadingCards = reading;
      void reading.then(remember, () => {}).finally(() => {
        if (libraryReadingCards !== reading) return;
        libraryReadingCards = undefined;
        finish();
      });
    } else {
      libraryReadingCards = undefined;
      finish();
    }
  }

  async function loadLibrary() {
    if (!invoke || libraryReady) return;
    if (libraryReading) return libraryReading;
    libraryReading = invoke("list_saved_decks").then((decks) => {
      savedDecks = decks;
      libraryReady = true;
      libraryDirty = true;
      renderLibrary();
    }).catch(showError).finally(() => {
      libraryReading = undefined;
      renderLoadState();
    });
    renderLoadState();
    return libraryReading;
  }

  function choosePriorityCollection() {
    const collections = workspace.collections;
    if (!collections.some((collection) => collection.id === primaryId)) {
      primaryId = (collections.find(({ name }) => deckCardKey(name) === "ma collection") ?? collections[0])?.id ?? 0;
    }
  }

  async function refresh() {
    if (!draft || !invoke || busy || (!stale && !loading)) return;
    if (loading) return loading;
    stale = false;
    let loaded = false;
    workspaceReading = true;
    renderLoadState();
    loading = invoke("load_deck_workspace").then((data) => {
      workspace = data;
      loaded = true;
      workspaceReading = false;
      remember(data.candidates);
      choosePriorityCollection();
      if (draft) void hydrateDeck(draft);
      renderDeck();
    }).catch((issue) => {
      stale = true;
      workspaceReading = false;
      showError(issue);
      if (draft) renderCards();
    }).finally(() => {
      loading = undefined;
      workspaceReading = false;
      renderLoadState();
      if (loaded && stale && !root.hidden) void refresh();
    });
    return loading;
  }

  function mayDiscard() {
    return !busy && (!dirty || window.confirm(t("Abandonner les modifications de ce deck ?")));
  }

  function openDeck(deck, modified = false) {
    closeActionMenu();
    resetPreview();
    tooltip.hidden = true;
    draft = deck ? structuredClone(deck) : undefined;
    landInputsSignature = undefined;
    controls["deck-name"].value = draft?.name ?? "";
    dirty = modified;
    showError();
    status.textContent = "";
    searchRevision++;
    searchInput.value = "";
    resultList.hidden = true;
    resultList.replaceChildren();
    renderLibrary();
    if (draft) {
      void refresh();
      void hydrateDeck(draft);
    }
    renderDeck(true);
  }

  async function hydrateDeck(opened) {
    if (!invoke || cardReading?.deck === opened) return;
    const canonicalize = () => {
      for (const card of opened.cards) card.name = info.get(deckCardKey(card.name))?.name ?? card.name;
    };
    const names = [...new Set([...opened.cards, ...savedDecks.flatMap((deck) => deck.cards)]
      .filter((card) => !info.has(deckCardKey(card.name))).map((card) => card.name))];
    canonicalize();
    if (!names.length) return;
    const reading = { deck: opened };
    cardReading = reading;
    renderLoadState();
    try {
      const cards = await invoke("search_deck_cards", { search: "", names, commander: false });
      remember(cards);
      canonicalize();
    } catch {
      // Imported quantities and card names remain editable while the catalog is unavailable.
    } finally {
      if (cardReading === reading) cardReading = undefined;
      if (draft === opened) renderDeck();
    }
  }

  function changed(resetBoard = false) {
    dirty = true;
    status.textContent = "";
    renderDeck(resetBoard);
  }

  function updateLandBuilder(stats) {
    const values = deckLandInputs(draft, info, stats?.recommendedLands);
    const signature = JSON.stringify(values);
    if (signature === landInputsSignature) return;
    landInputsSignature = signature;
    prefillLandBuilder(values);
  }

  function renderDeck(resetBoard = false) {
    controls["deck-editor"].hidden = !draft;
    if (!draft) {
      updateLandBuilder();
      refreshLibraryBadges();
      scheduleLegalities();
      renderLoadState();
      return;
    }
    const isCommander = draft.format === "commander";
    const nameInput = controls["deck-name"];
    if (document.activeElement !== nameInput) nameInput.value = draft.name;
    controls["deck-format"].value = draft.format;
    const commanders = draft.cards.filter((card) => card.board === "commander");
    controls["deck-commanders"].hidden = !isCommander;
    controls["deck-commanders"].textContent = commanders.map(cardLabel).join(" · ") || t(noCommander);
    const colors = new Set(deckColors(draft, info));
    for (const [color, input] of inputs) {
      input.checked = colors.has(color);
      input.disabled = isCommander;
    }
    const board = boardInput.value;
    const availableBoards = isCommander
      ? ["commander", "main", "sideboard", "considering"]
      : ["main", "sideboard", "considering"];
    boardInput.replaceChildren(...availableBoards.map((key) => option(key, BOARDS[key])));
    boardInput.value = !resetBoard && availableBoards.includes(board) ? board
      : isCommander && !commanders.length ? "commander" : "main";
    updateSearchLabel();
    const stats = deckStats(draft, info);
    updateLandBuilder(stats);
    const formatSize = t("{count} cartes · objectif {target}", {
      count: formatCount(stats.total), target: formatCount(stats.target),
    });
    const landSize = t("{count} terrains · {recommended} recommandés", {
      count: formatCount(stats.lands), recommended: formatCount(stats.recommendedLands),
    });
    controls["deck-progress"].textContent = [formatSize, landSize,
      t("{count} places pour les sorts", { count: formatCount(stats.spellSlots) }),
    ].join(" · ");
    root.querySelector('[data-deck-action="delete"]').hidden = !draft.id;
    setUiText(root.querySelector('[data-deck-action="back"]'), draft.id ? "Retour aux decks" : "Annuler");
    root.querySelector('[data-deck-action="save"]').disabled = busy || !draft.name.trim();
    root.querySelector('[data-deck-action="replace"]').disabled = busy;
    let inventory;
    const getInventory = () => (inventory ??= draftInventory());
    renderCards(getInventory);
    renderTokens();
    renderRecommendations(getInventory);
    scheduleLegalities();
    renderLoadState();
  }

  function dot(symbol, detail, tone = "") {
    const indicator = element("span", symbol, "deck-dot" + (tone ? " deck-dot--" + tone : ""));
    indicator.setAttribute("role", "img");
    indicator.setAttribute("aria-label", t(detail).replaceAll("\n", ". "));
    return indicator;
  }

  function collectionDot(result) {
    if (workspaceReading) return dot("…", "Chargement des collections", "loading");
    const holdings = new Map(result.holdings.map(({ collectionId, quantity }) => [collectionId, quantity]));
    const lines = [...workspace.collections]
      .sort((a, b) => Number(b.id === primaryId) - Number(a.id === primaryId))
      .map(({ id, name }) => name + " : " + formatCount(holdings.get(id) ?? 0));
    if (!lines.length) lines.push(t("Aucune collection enregistrée."));
    if (result.origins.length) lines.push(t("Affectation : {sources}", {
      sources: result.origins.map(({ collection, quantity }) => collection + " ×" + formatCount(quantity)).join(", "),
    }));
    if (result.missing) lines.push(t("{count} exemplaire(s) manquant(s)", { count: formatCount(result.missing) }));
    if (result.conflicts.length) lines.push(t("Dans : {decks}", { decks: result.conflicts.join(", ") }));
    const indicator = dot("C", lines.join("\n"), result.missing ? "warning" : "source");
    indicator.classList.add("deck-dot--collection");
    indicator.tabIndex = 0;
    const show = () => {
      tooltip.textContent = lines.join("\n");
      tooltip.hidden = false;
      const bounds = indicator.getBoundingClientRect();
      const left = Math.max(8, Math.min(bounds.left, document.documentElement.clientWidth - tooltip.offsetWidth - 8));
      const top = bounds.bottom + 6 + tooltip.offsetHeight <= window.innerHeight
        ? bounds.bottom + 6 : Math.max(8, bounds.top - tooltip.offsetHeight - 6);
      tooltip.style.left = left + "px";
      tooltip.style.top = top + "px";
    };
    indicator.addEventListener("mouseenter", show);
    indicator.addEventListener("focus", show);
    indicator.addEventListener("mouseleave", () => { tooltip.hidden = true; });
    indicator.addEventListener("blur", () => { tooltip.hidden = true; });
    return indicator;
  }

  function legalityDot(card, quantities) {
    const key = keyFor(card.name);
    const { warnings, status: legalityStatus } = deckWarnings(
      draft, card, info.get(key), info, legalities.get(key), quantities.get(key) ?? card.quantity,
    );
    const format = t(DECK_FORMATS[draft.format].label);
    let indicator;
    if (warnings.length) indicator = dot("×", warnings.map((warning) => t(warning)).join("\n"), "illegal");
    else if (legalityStatus === "legal") indicator = dot("✓", t("Légale en {format}", { format }), "legal");
    else if (legalityStatus === "restricted") indicator = dot("✓", t("Restreinte en {format} · un exemplaire autorisé", { format }), "legal");
    else if (draft.format === "limited") indicator = dot("–", "Légalité selon l’événement");
    else if (draft.format === "freeform") indicator = dot("–", "Format libre");
    else if (!checked.has(key)) indicator = dot("…", "Légalité en cours de vérification", "loading");
    else indicator = dot("?", "Légalité indisponible");
    indicator.dataset.deckLegality = "";
    return indicator;
  }

  function resetPreview() {
    window.clearTimeout(previewTimer);
    previewKey = undefined;
    clearCardImage(previewImage, previewState);
    previewName.textContent = "";
    setUiText(previewState, "Survolez le nom d’une carte.");
  }

  function cardNameButton(card, quantity) {
    const printing = () => resolveDeckPrinting(card, info.get(deckCardKey(card.name)));
    const preview = (delay) => {
      window.clearTimeout(previewTimer);
      previewTimer = window.setTimeout(() => {
        if (root.hidden) return;
        const item = printing();
        const key = [item.name, item.setCode, item.collectorNumber].join("\0");
        if (key === previewKey) return;
        previewKey = key;
        previewName.textContent = displayedCardName(item);
        void showCardImage(item, previewImage, previewState);
      }, delay);
    };
    const label = cardLabel(card);
    const button = action(label, () => showCardDetail({ ...printing(), quantity }), t("Fiche carte de {name}", { name: label }));
    button.className = "deck-card-name";
    button.addEventListener("mouseenter", () => preview(120));
    button.addEventListener("mouseleave", () => window.clearTimeout(previewTimer));
    button.addEventListener("focus", () => preview(0));
    return button;
  }

  function menuButton(label, open) {
    const trigger = action("⋯", () => open(trigger), label);
    trigger.className = "deck-card-actions-button";
    trigger.setAttribute("aria-haspopup", "true");
    trigger.setAttribute("aria-expanded", "false");
    trigger.setAttribute("aria-controls", cardMenu.id);
    return trigger;
  }

  function closeActionMenu(restoreFocus = false) {
    const trigger = menuTrigger;
    if (cardMenu.matches(":popover-open")) cardMenu.hidePopover();
    trigger?.setAttribute("aria-expanded", "false");
    if (restoreFocus) trigger?.focus({ preventScroll: true });
  }

  function openActionMenu(trigger, label, items) {
    if (cardMenu.matches(":popover-open") && menuTrigger === trigger) {
      closeActionMenu(true);
      return;
    }
    closeActionMenu();
    menuTrigger = trigger;
    cardMenu.setAttribute("aria-label", label);
    cardMenu.replaceChildren(...items.map(([name, apply]) => action(t(name), () => {
      closeActionMenu(true);
      if (!busy) void apply();
    })));
    cardMenu.showPopover();
    trigger.setAttribute("aria-expanded", "true");
    const rect = trigger.getBoundingClientRect();
    const { width, height } = cardMenu.getBoundingClientRect();
    cardMenu.style.left = Math.max(5, Math.min(rect.right - width, window.innerWidth - width - 5)) + "px";
    cardMenu.style.top = Math.max(5, rect.bottom + height <= window.innerHeight ? rect.bottom : rect.top - height) + "px";
    cardMenu.firstElementChild.focus({ preventScroll: true });
  }

  function increaseQuantity(card, amount) {
    if (!draft?.cards.includes(card) || busy) return;
    if (!Number.isInteger(amount) || amount < 1 || card.quantity + amount > 4294967295) {
      showError("Quantité trop élevée ou invalide.");
      return;
    }
    showError();
    card.quantity += amount;
    changed();
  }

  function openCardMenu(card, trigger) {
    const item = (label, apply) => [label, () => {
      if (draft?.cards.includes(card)) apply();
    }];
    const move = (board) => {
      if (moveDeckCard(draft.cards, card, board)) changed();
      else showError(board === "commander" ? "Ce commandant est déjà présent." : "Impossible de déplacer cette quantité.");
    };
    const items = [
      item("Ajouter 1", () => increaseQuantity(card, 1)),
      item("Ajouter plus…", () => {
        quantityTarget = card;
        quantityDialog.returnValue = "";
        controls["deck-quantity-add"].value = "2";
        quantityDialog.showModal();
        controls["deck-quantity-add"].select();
      }),
      item("Supprimer", () => {
        draft.cards.splice(draft.cards.indexOf(card), 1);
        changed();
      }),
    ];
    if (card.quantity > 1) {
      items.push(item("Retirer 1", () => {
        card.quantity--;
        changed();
      }));
    }
    if (card.board !== "main") items.push(item("Déplacer vers le deck principal", () => move("main")));
    if (card.board !== "sideboard") items.push(item("Déplacer vers le Sideboard", () => move("sideboard")));
    if (!isConsidering(card.board)) items.push(item("Déplacer vers Considering", () => move("considering")));
    if (draft.format === "commander" && card.board !== "commander") {
      items.push(item("Définir comme commandant", () => move("commander")));
    }
    openActionMenu(trigger, t("Actions pour {name}", { name: cardLabel(card) }), items);
  }

  function renderCards(getInventory = draftInventory) {
    const target = controls["deck-card-list"];
    const focused = target.contains(document.activeElement) ? document.activeElement.dataset.deckFocus : "";
    closeActionMenu();
    tooltip.hidden = true;
    target.replaceChildren();
    if (!draft.cards.length) {
      target.append(note("Ajoutez une carte pour commencer."));
      return;
    }
    const quantities = deckQuantities(draft.cards);
    const inventory = getInventory();
    const assigned = new Map();
    for (const { label, entries, quantity } of groupDeckCards(draft.cards, info)) {
      const group = element("section", null, "deck-card-group");
      group.append(element("h4", t(label) + " (" + formatCount(quantity) + ")"));
      const list = element("ul", null, "deck-card-list");
      const ordered = [...entries].sort((a, b) => compareLocalized(cardLabel(a.card), cardLabel(b.card)));
      for (const { card, index } of ordered) {
        const li = element("li", null, "deck-compact-row");
        li.dataset.deckCardIndex = String(index);
        const key = keyFor(card.name);
        const considering = isConsidering(card.board);
        const used = (considering ? quantities : assigned).get(key) ?? 0;
        const result = inventory(card.name, card.quantity, used);
        if (!considering) assigned.set(key, used + card.quantity);
        const name = cardNameButton(card, result.owned);
        name.dataset.deckFocus = "name-" + index;
        const trigger = menuButton(t("Actions pour {name}", { name: cardLabel(card) }), (button) => openCardMenu(card, button));
        trigger.dataset.deckFocus = "actions-" + index;
        li.append(
          element("span", formatCount(card.quantity), "deck-card-count"),
          name, collectionDot(result), legalityDot(card, quantities), trigger,
        );
        list.append(li);
      }
      group.append(list);
      target.append(group);
    }
    if (focused) (target.querySelector('[data-deck-focus="' + focused + '"]') ?? searchInput).focus();
  }

  function renderTokens() {
    const tokens = associatedDeckTokens(draft.cards, relatedTokens, keyFor);
    const section = controls["deck-associated-tokens"];
    section.hidden = !tokens.length;
    const signature = tokens.map((token) => [token.scryfallId, displayedCardName(token)].join("\0")).join("\0");
    if (signature === tokenSignature) return;
    tokenSignature = signature;
    const list = controls["deck-token-list"];
    clearImages(list);
    list.replaceChildren();
    for (const token of tokens) {
      const row = element("li");
      const figure = element("figure");
      const label = displayedCardName(token);
      const visual = imageFrame(label);
      figure.append(visual.frame, element("figcaption", label));
      row.append(figure);
      list.append(row);
      queueImage(visual, token);
    }
  }

  function addCard(card, board) {
    if (!draft || busy || (board === "commander" && draft.format !== "commander")) return;
    const key = keyFor(card.name);
    const entry = draft.cards.find((line) => keyFor(line.name) === key && line.board === board);
    const mainCard = board === "commander" && draft.cards.find((line) => line.board === "main" && keyFor(line.name) === key);
    if (entry) {
      if (board !== "commander" && entry.quantity < 4294967295) entry.quantity++;
    } else if (mainCard) {
      moveDeckCard(draft.cards, mainCard, "commander");
    } else {
      draft.cards.push({
        name: card.name, quantity: 1, board, setCode: card.setCode ?? "",
        collectorNumber: card.collectorNumber ?? "", cardType: card.typeLine ?? "",
      });
    }
    if (board === "commander") {
      boardInput.value = "main";
      searchInput.value = "";
      resultList.hidden = true;
      ++searchRevision;
    }
    changed();
    searchInput.focus({ preventScroll: true });
  }

  function renderChoices(target, cards, board, inventory = draftInventory()) {
    const list = element("ul", null, "deck-choice-list");
    for (const card of cards) {
      const row = element("li", null, "deck-card-row");
      const description = element("div");
      description.append(cardNameButton(card, inventory(card.name).owned), element("small", card.typeLine ?? ""));
      const command = board === "commander";
      const label = cardLabel(card);
      row.append(description, action(
        t(command ? "Définir comme commandant" : "Ajouter"), () => addCard(card, board),
        t(command ? "Définir {name} comme commandant" : "Ajouter {name}", { name: label }),
      ));
      list.append(row);
    }
    target.replaceChildren(list);
  }

  function renderRecommendations(getInventory = draftInventory) {
    const target = controls["deck-recommendations"];
    const stats = deckStats(draft, info);
    if (!stats.remaining) {
      target.replaceChildren(note("Objectif de taille atteint."));
      return;
    }
    if (draft.format === "commander" && !draft.cards.some((card) => card.board === "commander")) {
      target.replaceChildren(note("Choisissez un commandant pour orienter les propositions."));
      return;
    }
    const inventory = getInventory();
    const cards = recommendCards(draft, workspace.candidates, info, (name, copies) => inventory(name, 1, copies).available, legalities);
    if (cards.length) renderChoices(target, cards, "main", inventory);
    else target.replaceChildren(note("Aucune proposition disponible. Vous pouvez rechercher une carte par son nom."));
  }

  function updateSearchLabel() {
    setUiText(controls["deck-search-label"], boardInput.value === "commander"
      ? "Rechercher un commandant" : "Rechercher une carte à ajouter");
  }

  function runSearch() {
    window.clearTimeout(searchTimer);
    updateSearchLabel();
    const revision = ++searchRevision;
    const query = searchInput.value.trim();
    const board = boardInput.value;
    if (!draft || (query.length < 2 && board !== "commander")) {
      resultList.hidden = true;
      return;
    }
    resultList.hidden = false;
    if (query.length < 2) {
      const commanders = workspace.candidates.filter(canBeCommander).slice(0, 16);
      if (commanders.length) renderChoices(resultList, commanders, board);
      else resultList.replaceChildren(note("Saisissez au moins deux caractères."));
      return;
    }
    resultList.replaceChildren(note("Recherche…"));
    searchTimer = window.setTimeout(async () => {
      try {
        const cards = await invoke("search_deck_cards", { search: query, names: [], commander: board === "commander" });
        if (revision !== searchRevision) return;
        remember(cards);
        if (cards.length) renderChoices(resultList, cards, board);
        else resultList.replaceChildren(note("Aucune carte trouvée."));
      } catch (issue) {
        if (revision === searchRevision) resultList.replaceChildren(note(String(issue)));
      }
    }, 150);
  }

  function uncheckedTargets() {
    if (root.hidden || (draft ? cardReading?.deck === draft : libraryReadingCards)) return [];
    const decks = draft ? [draft] : libraryToCheck;
    const cards = new Map();
    for (const deck of decks) {
      for (const card of deck.cards) {
        const key = keyFor(card.name);
        if (checked.has(key) || cards.has(key) || (!draft && isConsidering(card.board))) continue;
        const { name, scryfallId, setCode, collectorNumber } = resolveDeckPrinting(card, info.get(key));
        cards.set(key, { name, scryfallId, setCode, collectorNumber });
        if (cards.size === 75) return [...cards];
      }
    }
    return [...cards];
  }

  function renderLoadState() {
    const messages = [];
    const readingCards = !!draft && cardReading?.deck === draft;
    if (libraryReading && !libraryReady && !draft) messages.push("Chargement des decks");
    if (draft && workspaceReading) messages.push("Chargement des collections");
    if (readingCards) messages.push("Chargement des cartes");
    if (draft && (checkingLegalities || uncheckedTargets().length)) messages.push("Vérification des légalités et des jetons");
    const target = controls["deck-load-state"];
    setUiText(target, () => messages.map((message) => t(message)).join(" · "));
    target.hidden = !messages.length;
    savedList.setAttribute("aria-busy", String(!!libraryReading && !libraryReady));
    controls["deck-card-list"].setAttribute("aria-busy", String(readingCards || checkingLegalities));
  }

  function refreshLegalityDots() {
    const quantities = deckQuantities(draft.cards);
    for (const row of controls["deck-card-list"].querySelectorAll("[data-deck-card-index]")) {
      const card = draft.cards[Number(row.dataset.deckCardIndex)];
      if (card) row.querySelector("[data-deck-legality]").replaceWith(legalityDot(card, quantities));
    }
  }

  function scheduleLegalities() {
    window.clearTimeout(legalityTimer);
    if (!invoke || checkingLegalities || !uncheckedTargets().length) return;
    legalityTimer = window.setTimeout(async () => {
      const batch = uncheckedTargets();
      if (!batch.length) return;
      checkingLegalities = true;
      renderLoadState();
      try {
        const result = await invoke("check_deck_legalities", { targets: batch.map(([, target]) => target) });
        for (const card of result) {
          const key = keyFor(card.name);
          legalities.set(key, card.legalities);
          relatedTokens.set(key, card.relatedTokens);
        }
      } catch {
        // A failed verification is represented as unavailable and never blocks editing.
      } finally {
        for (const [key, target] of batch) {
          checked.add(key);
          checked.add(keyFor(target.name));
        }
        checkingLegalities = false;
        if (!root.hidden) {
          if (draft) {
            refreshLegalityDots();
            renderTokens();
            renderRecommendations();
          } else {
            refreshLibraryBadges();
          }
        }
        scheduleLegalities();
        renderLoadState();
      }
    }, 200);
  }

  async function save() {
    if (!invoke || !draft || busy || !draft.name.trim()) return;
    const { id = null, revision = null, ...deck } = draft;
    const original = JSON.stringify(draft);
    const payload = structuredClone(deck);
    busy = true;
    showError();
    renderDeck();
    try {
      await libraryReading;
      const saved = await invoke("save_deck", { id, revision, deck: payload });
      savedDecks = [...savedDecks.filter((item) => item.id !== saved.id), saved];
      libraryDirty = true;
      dirty = JSON.stringify(draft) !== original;
      draft.id = saved.id;
      draft.revision = saved.revision;
      setUiText(status, dirty ? "Deck enregistré. D’autres modifications restent à enregistrer." : "Deck enregistré.");
      renderLibrary();
    } catch (issue) {
      stale = true;
      showError(issue);
    } finally {
      busy = false;
      renderDeck();
      if (stale && !root.hidden) void refresh();
    }
  }

  async function duplicateDeck({ name, format, colors, cards }) {
    if (!invoke || busy) return;
    busy = true;
    showError();
    const names = new Set(savedDecks.map((deck) => deckCardKey(deck.name)));
    let copyName = name.slice(0, 110) + " (copie)";
    for (let number = 2; names.has(deckCardKey(copyName)); number++) {
      copyName = name.slice(0, 100) + " (copie " + number + ")";
    }
    try {
      await libraryReading;
      const saved = await invoke("save_deck", {
        id: null, revision: null, deck: { name: copyName, format, colors, cards },
      });
      savedDecks.push(saved);
      libraryDirty = true;
      renderLibrary();
      setUiText(status, "Deck dupliqué.");
    } catch (issue) {
      showError(issue);
    } finally {
      busy = false;
    }
  }

  async function removeDeck(deck) {
    if (!invoke || !deck?.id || busy || !window.confirm(t("Supprimer le deck « {name} » ?", { name: deck.name }))) return;
    busy = true;
    showError();
    const { id, revision } = deck;
    try {
      await libraryReading;
      await invoke("delete_deck", { id, revision });
      savedDecks = savedDecks.filter((item) => item.id !== id);
      libraryDirty = true;
      if (draft?.id === id) openDeck();
      else renderLibrary();
      setUiText(status, "Deck supprimé.");
    } catch (issue) {
      stale = true;
      showError(issue);
    } finally {
      busy = false;
      if (stale && !root.hidden) void refresh();
    }
  }

  function selectImport(replace) {
    if (!invoke || !mayDiscard()) return;
    replaceImport = replace;
    fileInput.value = "";
    fileInput.click();
  }

  fileInput.addEventListener("change", async () => {
    const file = fileInput.files?.item(0);
    if (!file || busy) return;
    if (file.size > 20 * 1024 * 1024) {
      showError("Le fichier CSV dépasse 20 Mio.");
      return;
    }
    busy = true;
    showError();
    setUiText(status, "Lecture du fichier CSV…");
    try {
      const decks = await invoke("preview_deck_import", { csv: await file.text(), filename: file.name });
      pendingImport = { decks, replace: replaceImport };
      importChoice.replaceChildren(...decks.map((deck, index) => option(index, deck.name + " · " + t(DECK_FORMATS[deck.format].label))));
      setUiText(get("deck-import-title"), replaceImport ? "Remplacer le contenu du deck" : "Importer un deck");
      importDialog.showModal();
    } catch (issue) {
      showError(issue);
    } finally {
      busy = false;
      status.textContent = "";
    }
  });

  const handlers = {
    new: () => {
      if (!mayDiscard()) return;
      const names = new Set(savedDecks.map((deck) => deckCardKey(deck.name)));
      const base = t("Nouveau deck");
      let name = base;
      for (let number = 2; names.has(deckCardKey(name)); number++) name = base + " " + number;
      openDeck({ name, format: "standard", colors: [], cards: [] });
      controls["deck-name"].select();
    },
    import: () => selectImport(),
    replace: () => selectImport(draft),
    back: () => {
      if (!mayDiscard()) return;
      openDeck();
      root.querySelector('[data-deck-action="new"]').focus();
    },
    save,
    delete: () => removeDeck(draft),
    "cancel-import": () => importDialog.close(),
    "confirm-import": () => {
      const imported = pendingImport?.decks[Number(importChoice.value)];
      if (!imported) return;
      const replace = pendingImport.replace;
      const deck = replace
        ? { ...imported, id: replace.id, revision: replace.revision, name: replace.name,
            format: imported.format === "freeform" ? replace.format : imported.format, colors: replace.colors }
        : imported;
      importDialog.close();
      openDeck(deck, true);
      setUiText(status, "Vérifiez le deck, puis enregistrez-le.");
    },
  };
  for (const button of root.querySelectorAll("[data-deck-action]")) {
    button.addEventListener("click", handlers[button.dataset.deckAction]);
  }
  importDialog.addEventListener("close", () => { pendingImport = undefined; });
  quantityDialog.addEventListener("close", () => {
    if (quantityDialog.returnValue === "add" && quantityTarget) {
      increaseQuantity(quantityTarget, controls["deck-quantity-add"].valueAsNumber);
    }
    quantityTarget = undefined;
  });
  cardMenu.addEventListener("toggle", () => {
    if (!cardMenu.matches(":popover-open")) {
      menuTrigger?.setAttribute("aria-expanded", "false");
      menuTrigger = undefined;
    }
  });
  cardMenu.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      closeActionMenu(true);
      return;
    }
    const buttons = [...cardMenu.querySelectorAll("button")];
    const index = buttons.indexOf(document.activeElement);
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      buttons[(index + step + buttons.length) % buttons.length]?.focus();
    }
  });
  window.addEventListener("scroll", () => {
    closeActionMenu();
    tooltip.hidden = true;
  }, { capture: true, passive: true });
  controls["deck-name"].addEventListener("input", (event) => {
    draft.name = event.target.value;
    dirty = true;
    root.querySelector('[data-deck-action="save"]').disabled = busy || !draft.name.trim();
  });
  controls["deck-format"].addEventListener("change", (event) => {
    draft.format = event.target.value;
    changed(true);
    runSearch();
  });
  searchInput.addEventListener("input", runSearch);
  boardInput.addEventListener("change", runSearch);

  return {
    preload() {
      void loadLibrary();
    },
    open() {
      void loadLibrary();
      void refresh();
      if (languageDirty) {
        languageDirty = false;
        renderDeck();
        if (draft) runSearch();
      }
      if (draft) {
        refreshLegalityDots();
        renderTokens();
      } else {
        refreshLibraryBadges();
      }
      scheduleLegalities();
      renderLoadState();
    },
    close() {
      closeActionMenu();
      tooltip.hidden = true;
      if (previewKey || previewTimer) resetPreview();
    },
    invalidate(active) {
      stale = true;
      if (active) void refresh();
    },
    languageChanged() {
      languageDirty = true;
      libraryDirty = true;
    },
  };
}
