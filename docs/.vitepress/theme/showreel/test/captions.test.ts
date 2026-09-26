// The captions' reading rules (the storyboard's design/check/captions.py):
// every must-read line holds long enough to read, two-line captions hold all
// their words, and captions never share the screen. Checked for every
// scene's captions under today's figures, with exactly one claim, with a
// run that backs no claim, and with no facts at all.

import assert from "node:assert/strict";
import { test } from "node:test";
import { BEAT, PALETTE, type ReelFacts, type SectionId, sec } from "../bible";
import { races } from "../facts";
import { scenes } from "../scenes";
import { type Caption, CODE, entrance, PAPER, plain, readingTime, timeCaptions, WIPE, WORD, wordCount } from "../type";
import { noClaim, oneClaim, today } from "./published";

const VARIANTS: [string, ReelFacts | null][] = [
  ["facts", today()],
  ["one claim", oneClaim()],
  ["no claim", noClaim()],
  ["no facts", null],
];

/** Every scene's captions under each set of facts, in timeline order. */
function everyCaption(): { name: string; id: SectionId; caps: readonly Caption[] }[] {
  return VARIANTS.flatMap(([what, facts]) =>
    scenes.map((s) => ({ name: `${s.id} (${what})`, id: s.id, caps: s.captions?.(facts) ?? [] })),
  );
}

/** The first frame at or after `t` on a `fps` grid, as a frame number. */
const frameAt = (t: number, fps: number): number => Math.ceil(t * fps - 1e-6);

/**
 * Seconds a line is fully in on a `fps` grid: from the first frame that
 * shows its last word landed to the first frame of its wipe.
 */
function heldOnFrames(landed: number, out: number, fps: number): number {
  return (frameAt(out, fps) - frameAt(landed, fps)) / fps;
}

test("the storyboard's captions are on screen", () => {
  // §7.1: fourteen captions with a claim, in every section but morph and end.
  for (const [what, facts] of VARIANTS) {
    const n = scenes.reduce((k, s) => k + (s.captions?.(facts) ?? []).length, 0);
    // The race keeps two captions while any claim stands, and one without.
    assert.equal(n, races(facts).length ? 14 : 13, what);
    for (const s of scenes) {
      const has = (s.captions?.(facts) ?? []).length > 0;
      assert.equal(has, s.id !== "morph" && s.id !== "end", `${s.id} (${what})`);
    }
  }
});

test("captions have one or two lines of at most 36 characters, landing in order", () => {
  for (const { name, id, caps } of everyCaption()) {
    const beats = sec(id).bars * 4;
    for (const c of caps) {
      assert.ok(c.lines.length >= 1 && c.lines.length <= 2, `${name}: ${c.lines.length} lines`);
      c.lines.forEach((l, i) => {
        assert.ok(plain(l.text).length <= 36, `${name}: "${plain(l.text)}" is too long`);
        assert.equal((l.text.match(/`/g) ?? []).length % 2, 0, `${name}: unclosed code in "${l.text}"`);
        assert.ok(l.in < c.out, `${name}: "${l.text}" leaves before it lands`);
        if (i > 0) assert.ok(l.in >= c.lines[i - 1].in, `${name}: "${l.text}" lands before the line above it`);
        // The first word starts inside the section.
        assert.ok(entrance(l.text, l.in * BEAT) >= -1e-9, `${name}: "${l.text}" starts before its section`);
      });
      // The wipe finishes inside the section.
      assert.ok(c.out * BEAT + WIPE <= beats * BEAT + 1e-9, `${name}: the wipe at b${c.out} runs past the section`);
    }
  }
});

test("every in and out is on the half-beat grid, so holds are exact at 60 and 120 fps", () => {
  for (const { name, caps } of everyCaption()) {
    for (const c of caps) {
      assert.ok(Number.isInteger(c.out * 2), `${name}: out b${c.out}`);
      // F1's slot-two line lands on b9.75, a sixteenth: still a 120 fps frame, and its hold is checked on both grids below.
      for (const l of c.lines) assert.ok(Number.isInteger(l.in * 4), `${name}: "${l.text}" lands on b${l.in}`);
    }
  }
});

test("every line holds for words / 4 + 0.5 s, on the beat grid and on both frame grids", () => {
  for (const { name, id, caps } of everyCaption()) {
    const s = sec(id);
    for (const c of caps) {
      for (const l of c.lines) {
        const need = readingTime(wordCount(l.text));
        const landed = s.beat(l.in);
        const out = s.beat(c.out);
        assert.ok(out - landed >= need - 1e-9, `${name}: "${plain(l.text)}" holds ${(out - landed).toFixed(3)} s, needs ${need}`);
        for (const fps of [60, 120]) {
          const held = heldOnFrames(landed, out, fps);
          assert.ok(held >= need - 1e-9, `${name} at ${fps} fps: "${plain(l.text)}" holds ${held.toFixed(4)} s, needs ${need}`);
        }
      }
    }
  }
});

test("a two-line caption holds all its words from the moment its first line lands", () => {
  for (const { name, id, caps } of everyCaption()) {
    const s = sec(id);
    for (const c of caps) {
      const words = c.lines.reduce((n, l) => n + wordCount(l.text), 0);
      const need = readingTime(words);
      const first = s.beat(Math.min(...c.lines.map((l) => l.in)));
      const out = s.beat(c.out);
      const text = c.lines.map((l) => plain(l.text)).join(" / ");
      assert.ok(out - first >= need - 1e-9, `${name}: "${text}" holds ${(out - first).toFixed(3)} s, needs ${need}`);
      for (const fps of [60, 120]) {
        const held = heldOnFrames(first, out, fps);
        assert.ok(held >= need - 1e-9, `${name} at ${fps} fps: "${text}" holds ${held.toFixed(4)} s, needs ${need}`);
      }
    }
  }
});

test("captions never overlap: the next lands at least half a beat after the last starts to leave", () => {
  for (const { name, caps } of everyCaption()) {
    const sorted = [...caps].sort((a, b) => a.lines[0].in - b.lines[0].in);
    for (let i = 1; i < sorted.length; i++) {
      const prev = sorted[i - 1];
      const next = sorted[i];
      const lands = Math.min(...next.lines.map((l) => l.in));
      assert.ok(lands >= prev.out + 0.5, `${name}: a caption lands on b${lands}, the last leaves on b${prev.out}`);
    }
  }
  // On the reel's clock no caption's first word rises before the last
  // caption starts to wipe, so a new line never lands on one still whole.
  for (const [, facts] of VARIANTS) {
    const timed = scenes
      .flatMap((s) => timeCaptions(sec(s.id), s.captions?.(facts) ?? []))
      .sort((a, b) => a.start - b.start);
    for (let i = 1; i < timed.length; i++) {
      const text = timed[i].lines.map((l) => plain(l.text)).join(" / ");
      assert.ok(timed[i].start >= timed[i - 1].out - 1e-9, `"${text}" starts before the caption above it leaves`);
    }
  }
});

test("words land one per 1/32 note and leave over a sixteenth", () => {
  assert.equal(WORD, BEAT / 8);
  assert.equal(WIPE, BEAT / 4);
  // Code is in backticks and each run of it is one or more words.
  assert.equal(entrance("After `hk install`, `git commit`", 1), 1 - 5 * WORD);
  assert.equal(entrance("Builtins, from `prettier` to `zizmor`.", 1), 1 - 5 * WORD);
});

test("words are counted as captions.py counts them", () => {
  assert.equal(wordCount("After `hk install`, `git commit`"), 5);
  assert.equal(wordCount("so linters see only what you staged."), 7);
  assert.equal(wordCount("Fix every file: 1.3× faster"), 5);
  assert.equal(wordCount("Can't be fixed?"), 3);
  assert.equal(readingTime(4), 1.5);
  // At 120 BPM a line holds words / 2 + 1 beats.
  assert.equal(readingTime(10) / BEAT, 10 / 2 + 1);
});

test("the race states its ratios from the facts, and a line with no number without them", () => {
  const race = scenes.find((s) => s.id === "race");
  const text = (facts: ReelFacts | null) =>
    (race?.captions?.(facts) ?? []).flatMap((c) => c.lines.map((l) => plain(l.text))).join(" / ");
  assert.equal(
    text(today()),
    "Fix every file: 1.3× faster / than the fastest other tool. / Check every file: 1.8× faster / than the fastest other tool.",
  );
  assert.equal(text(oneClaim()), "Fix every file: 1.3× faster / than the fastest other tool. / Timed only when the files are right.");
  assert.equal(text(noClaim()), "Independent steps run in parallel.");
  assert.equal(text(null), "Independent steps run in parallel.");
});

test("the caption colours are the palette's", () => {
  assert.equal(PAPER, PALETTE.paper);
  assert.equal(CODE, PALETTE.warm);
});
