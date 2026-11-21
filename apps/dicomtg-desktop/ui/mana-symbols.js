const SYMBOL_LABELS = {
  W: "Mana blanc",
  U: "Mana bleu",
  B: "Mana noir",
  R: "Mana rouge",
  G: "Mana vert",
  C: "Mana incolore",
  S: "Mana neigeux",
  X: "Mana variable X",
  Y: "Mana variable Y",
  Z: "Mana variable Z",
  T: "Engagez ce permanent",
  Q: "Dégagez ce permanent",
  E: "Énergie",
};

const NUMBER_LABELS = {
  0: "Zéro mana générique",
  1: "Un mana générique",
  2: "Deux manas génériques",
};

export function manaSymbolUrl(symbol) {
  const match = /^\{([0-9A-Z∞/]+)\}$/u.exec(String(symbol ?? "").trim().toUpperCase());
  if (!match) return null;
  const filename = match[1].replaceAll("/", "").replaceAll("∞", "INFINITY");
  return `https://svgs.scryfall.io/card-symbols/${filename}.svg`;
}

function manaSymbolLabel(symbol) {
  const code = symbol.slice(1, -1).toUpperCase();
  if (SYMBOL_LABELS[code]) return SYMBOL_LABELS[code];
  if (/^\d+$/.test(code)) {
    return NUMBER_LABELS[code] ?? `${code} manas génériques`;
  }
  return `Symbole de mana ${symbol}`;
}

export function manaSymbolParts(value) {
  const text = String(value ?? "");
  const parts = [];
  let position = 0;
  for (const match of text.matchAll(/\{[^{}]+\}/gu)) {
    if (match.index > position) parts.push(text.slice(position, match.index));
    const symbol = match[0].toUpperCase();
    const url = manaSymbolUrl(symbol);
    if (url) parts.push({ symbol, label: manaSymbolLabel(symbol), url });
    else parts.push(match[0]);
    position = match.index + match[0].length;
  }
  if (position < text.length) parts.push(text.slice(position));
  return parts;
}
