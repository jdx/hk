// The song's clock and words (song.ts): the beats run forward at a shanty's
// tempo, the lines are in the order they are sung, every answer is the
// words the crew sings, and the shanty's page prints the same lyrics.

import assert from "node:assert/strict";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { REPO } from "../../test/repo";
import { answerAt, answerLength, bar, BEATS, beatAt, callAt, chaptersVtt, FIRST_BAR, LYRICS, plain, printed, SECTIONS, SONG, sungAt, timeOfBeat } from "../song";

test("the beats run forward, a beat every 0.45 to 0.55 s, across the whole song", () => {
  for (let i = 1; i < BEATS.length; i++) {
    const d = BEATS[i] - BEATS[i - 1];
    assert.ok(d >= 0.45 && d <= 0.55, `beat ${i} is ${d.toFixed(3)} s after the one before`);
  }
  // From the first boot to the last, within a beat of the recording's end.
  assert.ok(BEATS[0] > 0 && BEATS[0] < 0.55);
  assert.ok(BEATS[BEATS.length - 1] < SONG.duration && SONG.duration - BEATS[BEATS.length - 1] < 0.55);
  // Two pickups, then 4/4 from the first downbeat.
  assert.equal(FIRST_BAR, 2);
  assert.equal(bar(0), BEATS[2]);
  assert.equal(bar(107), BEATS[430]);
});

test("beatAt and timeOfBeat are each other's inverse, and exact on the beats", () => {
  BEATS.forEach((t, i) => {
    assert.equal(beatAt(t), i);
    assert.equal(timeOfBeat(i), t);
  });
  for (let t = -1; t < SONG.duration + 1; t += 0.137) assert.ok(Math.abs(timeOfBeat(beatAt(t)) - t) < 1e-9, `at ${t} s`);
});

test("sections run in order from 0, each starting before its first sung word", () => {
  assert.equal(SECTIONS[0].start, 0);
  assert.equal(new Set(SECTIONS.map((s) => s.id)).size, SECTIONS.length);
  SECTIONS.forEach((s, i) => {
    const next = SECTIONS[i + 1]?.start ?? SONG.duration;
    assert.ok(s.start < next, s.id);
    for (const l of LYRICS.filter((x) => x.section === s.id)) {
      for (const [w, at] of l.sung) assert.ok(at > s.start && at < next, `${l.id}: "${w}" at ${at} s is outside ${s.id}`);
    }
  });
});

test("the lines are sung in order, every word after the one before", () => {
  assert.equal(new Set(LYRICS.map((l) => l.id)).size, LYRICS.length);
  let last = 0;
  for (const l of LYRICS) {
    assert.ok(l.sung.length > 0, l.id);
    for (const [w, at] of l.sung) {
      assert.ok(at >= last, `${l.id}: "${w}" at ${at} s is sung before the word before it (${last} s)`);
      last = at;
    }
  }
  assert.ok(last < SONG.duration);
});

test("every answer is the last words sung, one sung word for each word shown", () => {
  for (const l of LYRICS) {
    assert.ok(l.call || l.answer, `${l.id} shows nothing`);
    const n = answerLength(l);
    assert.ok(n <= l.sung.length - (l.call ? 1 : 0), `${l.id}: its answer is more words than were sung after the call`);
    assert.equal(answerAt(l).length, n);
    if (l.call) assert.ok(callAt(l) < (answerAt(l)[0] ?? Infinity), l.id);
  }
  assert.equal(sungAt("verse-6.4", "twice"), 165.14);
  assert.throws(() => sungAt("verse-6.4", "thrice"));
});

test("docs/shanty.md prints every line, and the crew's answer in parentheses", () => {
  const page = readFileSync(join(REPO, "docs/shanty.md"), "utf8");
  for (const l of LYRICS) {
    if (l.call) assert.ok(page.includes(printed(l.call)), `${l.id}: the page does not print ${JSON.stringify(printed(l.call))}`);
    if (l.answer) {
      // "(Heave!)" and "(Ho!)" are one answer, printed a line each.
      for (const part of l.call ? [plain(l.answer)] : plain(l.answer).split(" ")) {
        assert.ok(page.includes(`_(${part})_`), `${l.id}: the page does not print the answer "(${part})"`);
      }
    }
  }
});

test("docs/public/bound-for-the-main-chapters.vtt is the chapters track SECTIONS generates", () => {
  const file = join(REPO, "docs/public/bound-for-the-main-chapters.vtt");
  if (process.env.UPDATE_CHAPTERS) writeFileSync(file, chaptersVtt());
  const served = existsSync(file) ? readFileSync(file, "utf8") : "";
  assert.equal(served, chaptersVtt(), "the chapters track is stale: run `UPDATE_CHAPTERS=1 aube run test:showreel` in docs/");
});
