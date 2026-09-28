// Everywhere's second idea, one step in two modes (storyboard §6.8): under
// hk check ruff-format reads the file under a read lock, fails and its diff
// unfolds; under hk fix the same command reads the file, which ruff has
// fixed first, under a read lock, then the step trades it for the write
// lock and hk applies its diff. The
// beats hand on in order, the caption's lines land on what they name, and
// the diff card stays on the stage and clear of the captions' band.

import assert from "node:assert/strict";
import { test } from "node:test";
import { STAGE, sec } from "../bible";
import { RUFF_FORMAT_DIFF, RUFF_FORMAT_FIX_DIFF } from "../kit/card";
import { HANDOFF_GAP } from "../kit/lanes";
import { CAPTIONS, COLUMNS, DIFF } from "../scenes/everywhere";
import { ALL_DONE, APPLY, CHECK_DONE, CHECK_GO, COLUMN_DRAW, DIFF_IN, DIFF_ROW, FIX_DIFF_ROW, FIX_DONE, FIX_GO, FIX_PATCH, FIX_WRITE } from "../scenes/everywhere-timing";
import { CAPTION_TOP } from "../type";
import { WHIP_START } from "../whip";

test("check fails and its diff unfolds; fix runs the same command on ruff's output, then writes its diff", () => {
  for (const c of COLUMNS) assert.ok(c.at + COLUMN_DRAW <= c.holds[0].from, `${c.title} has drawn in before its step starts`);
  assert.ok(CHECK_GO < CHECK_DONE);
  assert.equal(DIFF_IN, CHECK_DONE, "the diff unfolds from the ✗");
  assert.equal(DIFF_IN + RUFF_FORMAT_DIFF.length * DIFF_ROW, FIX_GO, "whole just as fix starts");
  assert.equal(FIX_PATCH - FIX_GO, CHECK_DONE - CHECK_GO, "the same command reads for as long in both");
  assert.equal(FIX_WRITE - FIX_PATCH, 1 / 4, "the write lock shuts a sixteenth after the read lock lets go, as on the lanes");
  assert.equal(FIX_PATCH + RUFF_FORMAT_FIX_DIFF.length * FIX_DIFF_ROW, FIX_WRITE, "its diff is whole as the write lock shuts");
  assert.ok(FIX_WRITE < APPLY[0] && APPLY[1] < FIX_DONE, "hk applies it while the step holds the write lock");
  assert.ok(FIX_DONE <= ALL_DONE);
  // All still before the whip winds up (everywhere b10.5).
  assert.ok(sec("everywhere").beat(ALL_DONE + 0.5) <= WHIP_START + 1e-9);
});

test("each column's step holds the file as the run did: check reads it; fix reads it, then writes it", () => {
  const [check, fix] = COLUMNS;
  const holds = (c: (typeof COLUMNS)[number]) => c.holds.map((h) => `${h.lock} b${h.from}–b${h.to} ${h.x0}–${h.x1}`);
  assert.deepEqual(holds(check), [`read b${CHECK_GO}–b${CHECK_DONE} 540–720`]);
  assert.deepEqual(holds(fix), [`read b${FIX_GO}–b${FIX_PATCH} 1400–1580`, `write b${FIX_WRITE}–b${FIX_DONE} ${1580 + HANDOFF_GAP}–1760`]);
  // The reads' pills are the same length: the same command.
  assert.equal(check.holds[0].x1 - check.holds[0].x0, fix.holds[0].x1 - fix.holds[0].x0);
  for (const c of COLUMNS) {
    for (const h of c.holds) assert.ok(h.x0 >= c.x0 && h.x1 <= c.x1 && h.x0 < h.x1, `${c.title}: inside its track`);
    for (let i = 1; i < c.holds.length; i++) assert.ok(c.holds[i].from > c.holds[i - 1].to, `${c.title}: never two holds at once`);
  }
});

test("the caption's lines land on the check and on fix's diff", () => {
  const [, modes] = CAPTIONS;
  assert.deepEqual(
    modes.lines.map((l) => l.text),
    ["Check shows the diff.", "Fix applies it."],
  );
  assert.equal(modes.lines[0].in, CHECK_GO);
  assert.equal(modes.lines[1].in, FIX_PATCH, "as fix has its diff");
});

test("the diff card, at its tallest, is on the stage and clear of the captions' band", () => {
  const h = 2 * DIFF.pad + Math.max(RUFF_FORMAT_DIFF.length, RUFF_FORMAT_FIX_DIFF.length) * DIFF.rowH;
  assert.ok(DIFF.y + h <= CAPTION_TOP - 8, `card bottom ${DIFF.y + h}`);
  for (const c of COLUMNS) {
    const x0 = c.x + DIFF.dx;
    assert.ok(x0 >= STAGE.left && x0 + DIFF.w <= STAGE.right, `${c.title}: x ${x0}–${x0 + DIFF.w}`);
    // Its right edge is the step's track's: the diff hangs from the step.
    assert.equal(x0 + DIFF.w, c.x1, c.title);
  }
  // The longest row fits inside the card with its inset on both sides.
  const longest = Math.max(...[...RUFF_FORMAT_DIFF, ...RUFF_FORMAT_FIX_DIFF].map((l) => l.length));
  assert.ok(2 * DIFF.inset + longest * 0.6 * DIFF.size <= DIFF.w, `${longest} columns`);
});
