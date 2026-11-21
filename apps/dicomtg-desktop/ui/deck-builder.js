export const DECK_COLOR_CODES = ["W", "U", "B", "R", "G"];

function nonNegativeNumber(value) {
  const number = Number(value);
  return Number.isFinite(number) ? Math.max(0, number) : 0;
}

export function calculateBasicLands({
  basicLandSlots = 0,
  multicolorLands = {},
  coloredCards = {},
} = {}) {
  const slots = nonNegativeNumber(basicLandSlots);
  const lands = Object.fromEntries(
    DECK_COLOR_CODES.map((color) => [
      color,
      nonNegativeNumber(multicolorLands[color]),
    ]),
  );
  const cards = Object.fromEntries(
    DECK_COLOR_CODES.map((color) => [
      color,
      nonNegativeNumber(coloredCards[color]),
    ]),
  );
  const totalCards = Object.values(cards).reduce((total, value) => total + value, 0);
  const totalLandSources =
    slots + Object.values(lands).reduce((total, value) => total + value, 0);
  const recommendations = Object.fromEntries(
    DECK_COLOR_CODES.map((color) => [
      color,
      totalCards === 0
        ? 0
        : (cards[color] / totalCards) * totalLandSources - lands[color],
    ]),
  );

  return { totalCards, totalLandSources, recommendations };
}
