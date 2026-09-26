// The stash scene's beats (storyboard §6.4) as the score hears them: the
// main.py card reaches its place on b1.5, where the score's pat is, and is
// still before the blade cuts on b2; and the squiggles underline exactly the
// staged lines that ruff and ruff-format change, one per eighth.

import assert from "node:assert/strict";
import { test } from "node:test";
import { MAIN_PY_FIXED, MAIN_PY_STAGED } from "../kit/card";
import { cardRise } from "../scenes/stash";
import { CUT } from "../scenes/stash-peel";
import { CARD_IN, CARD_OUT, HOLD, SETTLED, SQUIGGLE_DRAW, SQUIGGLES } from "../scenes/stash-timing";

const STEP = 1 / 240;
const beats = (from: number, to: number) => Array.from({ length: Math.round((to - from) / STEP) + 1 }, (_, i) => from + i * STEP);

test("the card rises all the way to its place and reaches it on CARD_IN.to", () => {
  assert.equal(cardRise(CARD_IN.from), 0);
  let prev = 0;
  for (const b of beats(CARD_IN.from, CARD_IN.to - STEP)) {
    const v = cardRise(b);
    assert.ok(v >= prev && v < 1, `b${b}: ${v} after ${prev}`);
    prev = v;
  }
  assert.ok(Math.abs(cardRise(CARD_IN.to) - 1) < 1e-12);
});

test("the card is still moving as it lands, carries on a few pixels and is home by CARD_IN.settled", () => {
  const h = 1e-6;
  const left = (cardRise(CARD_IN.to) - cardRise(CARD_IN.to - h)) / h;
  const right = (cardRise(CARD_IN.to + h) - cardRise(CARD_IN.to)) / h;
  assert.ok(left > 0.1, `arrives at ${left} rest-heights a beat`);
  assert.ok(Math.abs(left - right) < 1e-3, `no kink on landing: ${left} then ${right}`);
  // The overshoot: above rest, by at most 2% of the 836 px rise (about 17 px).
  const over = beats(CARD_IN.to + STEP, CARD_IN.settled - STEP).map(cardRise);
  assert.ok(over.every((v) => v > 1 && v < 1.02));
  // Home, exactly and still, from `settled` until the card goes, and before the cut.
  assert.ok(CARD_IN.settled < CUT.from);
  for (const b of beats(CARD_IN.settled, CARD_OUT.from)) assert.equal(cardRise(b), 1);
});

test("the squiggles underline the staged lines ruff and ruff-format change, whole, one per eighth", () => {
  const changed = MAIN_PY_STAGED.flatMap((line, row) => (line.trim() && !MAIN_PY_FIXED.includes(line) ? [row] : []));
  assert.deepEqual(changed, [0, 1, 2, 4]);
  assert.deepEqual(
    SQUIGGLES.map((q) => q.row),
    changed,
  );
  for (const q of SQUIGGLES) {
    const line = MAIN_PY_STAGED[q.row];
    assert.equal(q.col0, line.length - line.trimStart().length, line);
    assert.equal(q.col1, line.length, line);
  }
  SQUIGGLES.forEach((q, i) => {
    assert.equal(q.at % 0.5, 0, `squiggle ${i} on the eighths`);
    if (i) assert.equal(q.at - SQUIGGLES[i - 1].at, 0.5);
  });
  assert.ok(SQUIGGLES[SQUIGGLES.length - 1].at + SQUIGGLE_DRAW <= HOLD.fade);
});

test("the scene settles on the stash|lanes handoff before its end", () => {
  assert.ok(SETTLED <= 12);
});
