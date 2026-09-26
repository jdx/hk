// Every bar line's handoff (storyboard §5): one per boundary, in order, on
// its bar line, holds except the whip, and each scene starting from the one
// it inherits and ending on the one it owes, and the storyboard's numbers
// for the frames the kit has no preset for. handoff-frames.test.ts draws
// the frames and holds every scene to them.

import assert from "node:assert/strict";
import { test } from "node:test";
import { type LitRect, SECTIONS, type SectionId, sec } from "../bible";
import { BOUNDARIES, CAPSULES, capsuleAt, COMMIT_STASH_LINES, END_STROKES, HANDOFFS, handoffIn, handoffOut } from "../handoff";
import { header, PANE_FULL } from "../kit/term";
import { scenes } from "../scenes";
import { WHIP_AT, WHIP_END, WHIP_START, whipIn, whipOut } from "../whip";

test("every bar line has a handoff, in order, on its bar line", () => {
  assert.equal(BOUNDARIES.length, SECTIONS.length - 1);
  assert.equal(Object.keys(HANDOFFS).length, 10);
  assert.deepEqual(Object.keys(HANDOFFS), BOUNDARIES);
  BOUNDARIES.forEach((id, i) => {
    const h = HANDOFFS[id];
    assert.ok(h, `no handoff for ${id}`);
    assert.equal(h.id, id);
    assert.equal(h.from, SECTIONS[i].id);
    assert.equal(h.to, SECTIONS[i + 1].id);
    assert.equal(h.t, sec(h.to).start);
    assert.equal(h.t, sec(h.from).end);
    assert.ok(h.note.length > 20, `${id} says what is on its frame`);
  });
  for (const { id } of SECTIONS) {
    const i = SECTIONS.findIndex((s) => s.id === id);
    assert.equal(handoffIn(id)?.to ?? null, i === 0 ? null : id);
    assert.equal(handoffOut(id)?.from ?? null, i === SECTIONS.length - 1 ? null : id);
  }
});

test("every bar line holds but the whip into the race", () => {
  for (const id of BOUNDARIES) assert.equal(HANDOFFS[id].meet, id === "everywhere|race" ? "motion" : "hold", id);
});

test("the whip crosses the race's bar line: out of shot on it, in shot on either side", () => {
  assert.equal(WHIP_AT, sec("race").start);
  assert.equal(WHIP_AT, HANDOFFS["everywhere|race"].t);
  // Wind-up 1.5 beats before (everywhere b10.5), cleared a beat after (race b1).
  assert.equal(WHIP_START, sec("everywhere").beat(10.5));
  assert.equal(WHIP_END, sec("race").beat(1));
  assert.equal(whipOut(WHIP_START), 0);
  assert.ok(whipOut(WHIP_AT) <= -1920, "the outgoing content is gone on the bar line");
  assert.ok(whipOut(WHIP_AT - 1 / 120) > -1920, "and still in shot on the frame before it");
  assert.ok(whipIn(WHIP_AT) >= 1760, "the incoming content is off the right edge on the bar line");
  assert.equal(whipIn(WHIP_END), 0);
});

test("commit|stash is the prompt, the header at 0/7 and the four staged files", () => {
  assert.deepEqual(COMMIT_STASH_LINES, [
    '$ git commit -m "feat: hoist the sails"',
    header("pre-commit", "fix", 0, 7),
    "✔ files - Fetching staged files (4 files)",
  ]);
  assert.equal(COMMIT_STASH_LINES[1], `hk 2.2.0 by @jdx – pre-commit – fix  [${" ".repeat(37)}] 0/7`);
});

test("race|morph's capsules span x 700–1420, 56 px high, on rows 270–630; morph|end drops only the barb", () => {
  assert.deepEqual(CAPSULES.rows, [270, 390, 510, 630]);
  CAPSULES.rows.forEach((y, i) => {
    const c = capsuleAt(i);
    // Round caps reach half the width past each cap centre.
    assert.equal(c.a.x - c.width / 2, 700);
    assert.equal(c.b.x + c.width / 2, 1420);
    assert.equal(c.a.y, y);
    assert.equal(c.b.y, y);
    assert.equal(c.width, 56);
    assert.deepEqual([c.color, c.alpha], i === 0 ? ["#4adef0", 1] : ["#8a9eac", 0.35]);
  });
  assert.deepEqual(END_STROKES, [1, 1, 1, 1, 1, 0]);
});

test("each scene returns its handoffs' lit screen on its side of the bar line", () => {
  // A screen at alpha 0 is no screen: the vignette covers it either way.
  const norm = (r: LitRect | null | undefined) => (r && r.alpha > 0 ? { x: r.x, y: r.y, w: r.w, h: r.h, alpha: +r.alpha.toFixed(6) } : null);
  const scene = (id: SectionId) => scenes.find((s) => s.id === id)!;
  for (const id of BOUNDARIES) {
    const h = HANDOFFS[id];
    const want = norm(h.lit);
    const out = scene(h.from);
    assert.deepEqual(norm(out.lit?.(out.end - out.start - 1 / 120)), want, `${h.from}'s last frame is lit as ${id} is`);
    assert.deepEqual(norm(scene(h.to).lit?.(0)), want, `${h.to}'s first frame is lit as ${id} is`);
  }
  assert.deepEqual(HANDOFFS["commit|stash"].lit, { x: PANE_FULL.x, y: PANE_FULL.y, w: PANE_FULL.w, h: PANE_FULL.h, alpha: 1 });
  assert.deepEqual(
    BOUNDARIES.filter((id) => HANDOFFS[id].lit),
    ["commit|stash"],
  );
});
