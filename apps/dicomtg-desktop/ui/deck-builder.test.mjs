import assert from "node:assert/strict";
import test from "node:test";

import { calculateBasicLands } from "./deck-builder.js";

test("reproduces the Power BI basic-land calculation", () => {
  const result = calculateBasicLands({
    basicLandSlots: 24,
    multicolorLands: { W: 8, U: 0, B: 8, R: 7, G: 0 },
    coloredCards: { W: 30, U: 0, B: 24, R: 23, G: 0 },
  });

  assert.equal(result.totalCards, 77);
  assert.equal(result.totalLandSources, 47);
  assert.ok(Math.abs(result.recommendations.W - 10.3116883117) < 1e-9);
  assert.ok(Math.abs(result.recommendations.B - 6.6493506494) < 1e-9);
  assert.ok(Math.abs(result.recommendations.R - 7.038961039) < 1e-9);
  assert.ok(
    Math.abs(
      Object.values(result.recommendations).reduce(
        (total, value) => total + value,
        0,
      ) - 24,
    ) < 1e-9,
  );
});

test("returns zero recommendations when no colored card is entered", () => {
  assert.deepEqual(calculateBasicLands({ basicLandSlots: 24 }).recommendations, {
    W: 0,
    U: 0,
    B: 0,
    R: 0,
    G: 0,
  });
});

test("ignores invalid and negative inputs", () => {
  const result = calculateBasicLands({
    basicLandSlots: -4,
    multicolorLands: { W: Number.NaN },
    coloredCards: { W: 10 },
  });

  assert.equal(result.totalLandSources, 0);
  assert.equal(result.recommendations.W, 0);
});
