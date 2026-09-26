import assert from "node:assert/strict";
import { test } from "node:test";
import { BAR, BEAT, CHAPTERS, DURATION, SECTIONS, sec } from "../bible";
import { POSTER_TIME } from "../reel";
import { scenes } from "../scenes";

test("sections run end to end in whole bars, from 0 to DURATION", () => {
  let t = 0;
  for (const { id, bars } of SECTIONS) {
    const s = sec(id);
    assert.ok(Number.isInteger(bars) && bars > 0, `${id}: ${bars} bars`);
    assert.equal(s.start, t, `${id} starts where the previous section ends`);
    assert.equal(s.len, bars * BAR);
    assert.equal(s.end, s.start + s.len);
    t = s.end;
  }
  assert.equal(t, DURATION);
});

test("sec() places beats, bars, and local times from the section's start", () => {
  for (const { id } of SECTIONS) {
    const s = sec(id);
    assert.equal(s.beat(0), s.start);
    assert.equal(s.bar(0), s.start);
    assert.equal(s.at(0), s.start);
    assert.equal(s.beat(4 * s.bars), s.end);
    assert.equal(s.bar(s.bars), s.end);
    assert.equal(s.beat(1.5) - s.start, 1.5 * BEAT);
  }
  assert.throws(() => sec("nope" as never));
});

test("chapters mirror the sections", () => {
  assert.deepEqual(
    CHAPTERS,
    SECTIONS.map(({ id }) => {
      const { label, start, end } = sec(id);
      return { id, label, start, end };
    }),
  );
});

test("one scene per section, in order, on its section's span", () => {
  assert.deepEqual(
    scenes.map((s) => s.id),
    SECTIONS.map((s) => s.id),
  );
  for (const scene of scenes) {
    const s = sec(scene.id);
    assert.equal(scene.start, s.start, scene.id);
    assert.equal(scene.end, s.end, scene.id);
  }
});

test("the reel is eleven sections: 32 bars at 120 BPM, 64.000 s", () => {
  assert.equal(BEAT, 0.5);
  assert.equal(BAR, 2);
  assert.deepEqual(
    SECTIONS.map(({ id, bars }) => `${id} ${bars}`),
    ["open 2", "config 3", "commit 3", "stash 3", "lanes 4", "restore 3", "catch 3", "everywhere 3", "race 4", "morph 1", "end 3"],
  );
  assert.equal(SECTIONS.reduce((n, s) => n + s.bars, 0), 32);
  assert.equal(DURATION, 64);
  // Every bar line is a whole second, so it is a frame at 60 and 120 fps.
  for (const { id } of SECTIONS) assert.ok(Number.isInteger(sec(id).start), id);
  // Each section's span, seconds.
  assert.deepEqual(
    SECTIONS.map(({ id }) => `${sec(id).start}-${sec(id).end}`),
    ["0-4", "4-10", "10-16", "16-22", "22-30", "30-36", "36-42", "42-48", "48-56", "56-58", "58-64"],
  );
});

test("no chapter label carries a number, so the chapters track never goes stale", () => {
  for (const { id, label } of SECTIONS) assert.doesNotMatch(label, /\d/, id);
});

test("the poster is catch at beat 8.5: 40.25 s", () => {
  assert.equal(POSTER_TIME, 40.25);
  assert.equal(POSTER_TIME, sec("catch").beat(8.5));
});
