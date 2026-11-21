export const PAGE_IDS = [
  "overview",
  "card",
  "types",
  "keywords",
  "sets",
  "rarities",
  "collection",
  "deck",
  "data",
];

export function pageFromHash(hash) {
  const page = hash.replace(/^#/, "");
  return PAGE_IDS.includes(page) ? page : "overview";
}
