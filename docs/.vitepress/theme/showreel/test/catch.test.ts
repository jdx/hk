// The catch (storyboard §6.7), checked on its geometry and its clock: the
// commit card's loop rides in under the inset without crossing its edge,
// the `main` label ends over the head it still names rather than over the
// parent, and the beats, frame table and swing the score reads
// (scenes/catch-timing.ts) are the ones the picture moves on.

import assert from "node:assert/strict";
import { test } from "node:test";
import { BEAT } from "../bible";
import { MAIN } from "../kit/mainline";
import { blocked } from "../kit/screens";
import { advance, firstLine, termLayout } from "../kit/term";
import { cardAt, insetPane, labelX, PANE, SNAG, STOP, SWING } from "../scenes/catch";
import { HOLE_Y, LOOP_R, LOOP_W } from "../scenes/catch-rig";
import { BEATS, FRAME_BEATS, swingAt } from "../scenes/catch-timing";

const DEG = Math.PI / 180;
/** Clear stage the loop keeps under the inset's bottom edge, px. */
const CLEAR = 12;

/**
 * The loop ring's ink on the stage while the card rides (the swing is 0 until
 * the yank): the hole turned with the card's pitch about its low corner, the
 * ring turned about the hole by the pitch and the sway, and the shake.
 */
function loopInk(lt: number): { left: number; right: number; top: number } | null {
  const card = cardAt(lt);
  if (!card) return null;
  const x = card.cx - card.w / 2;
  const top = card.bottom - card.h;
  const px = card.pitch > 0 ? x + card.w : x;
  const p = card.pitch * DEG;
  const hx = card.cx - px;
  const hy = top + HOLE_Y - card.bottom;
  const hole = { x: px + hx * Math.cos(p) - hy * Math.sin(p) + card.shake[0], y: card.bottom + hx * Math.sin(p) + hy * Math.cos(p) + card.shake[1] };
  const turn = (card.pitch + card.look.sway) * DEG;
  const c = { x: hole.x + LOOP_R * Math.sin(turn), y: hole.y - LOOP_R * Math.cos(turn) };
  const r = LOOP_R + LOOP_W / 2;
  return { left: c.x - r, right: c.x + r, top: c.y - r };
}

test("the card's loop rides in under the inset and never crosses its bottom edge", () => {
  let under = 0;
  let nearest = Number.POSITIVE_INFINITY;
  for (let lt = BEATS.ride * BEAT; lt < SNAG; lt += 1 / 240) {
    const ink = loopInk(lt);
    const { pane, alpha } = insetPane(lt);
    if (!ink || alpha <= 0 || ink.right < pane.x || ink.left > pane.x + pane.w) continue;
    under++;
    const gap = ink.top - (pane.y + pane.h);
    nearest = Math.min(nearest, gap);
    assert.ok(gap >= CLEAR, `b${(lt / BEAT).toFixed(3)}: the loop's top is ${gap.toFixed(1)} px under the inset's edge (want ${CLEAR})`);
  }
  assert.ok(under > 40, "the loop does pass under the inset");
  assert.ok(nearest < 20, `the loop rides close under the inset, ${nearest.toFixed(1)} px, in its shadow`);
});

test("the trimmed inset holds every blocked frame whole, with a margin under its last row", () => {
  assert.deepEqual([PANE.x, PANE.y, PANE.w], [120, 110, 760], "the storyboard's INSET x 120–880 from y 110");
  assert.equal(FRAME_BEATS.length, blocked.length, "a beat for each of blocked's frames");
  for (const lines of blocked) {
    assert.ok(lines.length <= PANE.rows, `${lines.length} rows fit the pane's ${PANE.rows}`);
    assert.equal(firstLine(PANE, lines.length), 0, "nothing scrolls");
    const last = termLayout(PANE, lines.length).baseline(lines.length - 1);
    assert.ok(PANE.y + PANE.h - last >= PANE.baseline0 - PANE.y - 12, "the last row sits as far off the bottom as the first sits off the top, give or take");
  }
});

test("the `main` label glides from the line's start to sit centred over the head", () => {
  const width = advance(40) * MAIN.label.text.length;
  assert.equal(labelX(0), MAIN.label.x, "restore|catch: at the line's start on b0");
  const panned = BEATS.panned * BEAT;
  for (const lt of [panned, SWING, 8.5 * BEAT]) {
    assert.ok(Math.abs(labelX(lt) + width / 2 - 600) < 1e-9, "centred over ada2ca4, panned to x 600");
    const parent = MAIN.parent.x - 800;
    assert.ok(labelX(lt) > parent + MAIN.dotR + 40, "clear of the parent, f92f487, at x 200");
  }
  for (let lt = 0; lt < panned; lt += 1 / 240) assert.ok(labelX(lt + 1 / 240) >= labelX(lt), "it never doubles back");
});

test("the score's clock is the picture's: beats, frame table and swing", () => {
  assert.equal(STOP, BEATS.stop * BEAT);
  assert.equal(SNAG, BEATS.snag * BEAT);
  assert.equal(SWING, BEATS.swing * BEAT);
  for (let i = 1; i < FRAME_BEATS.length; i++) assert.ok(FRAME_BEATS[i] > FRAME_BEATS[i - 1], "frames in order");
  assert.equal(FRAME_BEATS[11], BEATS.stop, "frame 11, ✗ shellcheck, lands on the ✗");
  // The poster (b8.5) is at an apex of the swing, about +2.0° (storyboard §8):
  // the damping brings the apex itself 25 ms early, and the card hangs still.
  const poster = 8.5 * BEAT;
  assert.ok(Math.abs(swingAt(poster) - 2.0) < 0.05, `θ(b8.5) = ${swingAt(poster).toFixed(3)}°`);
  let apex = poster;
  for (let lt = 8 * BEAT; lt <= 9 * BEAT; lt += 0.001) if (swingAt(lt) > swingAt(apex)) apex = lt;
  assert.ok(Math.abs(apex - poster) < 0.03, `the apex is ${((poster - apex) * 1000).toFixed(0)} ms before the poster`);
  assert.equal(swingAt(BEATS.yank * BEAT), 0, "still until the yank has the card off the line");
});
