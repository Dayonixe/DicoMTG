import { finishLabel } from "./formatters.js";
import { displayedCardName, displayedSetName } from "./game-localization.js";
import { t } from "./interface-language.js";

const columns = {
  name: ["Carte", displayedCardName],
  setCode: ["Extension", (card) => displayedSetName(card.setCode)],
  collectorNumber: ["Numéro", (card) => card.collectorNumber],
  quantity: ["Qté", (card) => card.quantity],
  finish: ["Finition", (card) => finishLabel(card.finish)],
  rarity: ["Rareté", (card) => card.rarity],
  colors: ["Couleurs", (card) => card.colors.join(", ")],
  manaValue: ["Mana", (card) => card.manaValue],
  priceCents: ["Prix (€)", (card) =>
    card.priceCents == null ? "" : (card.priceCents / 100).toFixed(2).replace(".", ",")],
};

function csvCell(value) {
  if (typeof value === "number") return String(value).replace(".", ",");
  let text = String(value ?? "");
  // Imported text must stay literal when opened in a spreadsheet.
  if (/^[\t\r\n]|^\s*[=+\-@＝＋－＠]/u.test(text)) text = "'" + text;
  return /[;"\r\n]/.test(text) ? '"' + text.replaceAll('"', '""') + '"' : text;
}

export function cardsTableCsv(items, columnKeys) {
  const fields = columnKeys.map((key) => columns[key]);
  const header = fields.map(([name]) => csvCell(t(name))).join(";");
  const rows = items.map((item) => fields.map(([, value]) => csvCell(value(item))).join(";"));
  return "\uFEFF" + [header, ...rows].join("\r\n") + "\r\n";
}
