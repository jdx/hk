// The music video's plan (video.ts): its shots cover the song end to end,
// and its words are up while they are sung. A call lands before its first
// word is sung and stays until its last has been; each answer lands word by
// word as the crew sings it and holds a moment after; and no two lines
// write over each other.

import assert from "node:assert/strict";
import { test } from "node:test";
import { entrance, WIPE, WORD } from "../../type";
import { ANSWER_HOLD, LEAD, lyricEnd, lyricStart, timeLyrics } from "../lyrics";
import { LYRICS, lyric, SONG } from "../song";
import { PLACE, SHOTS, shotAt, TIMED } from "../video";

test("the shots run end to end, from the first frame to the song's last", () => {
  assert.equal(SHOTS[0].start, 0);
  SHOTS.forEach((s, i) => {
    assert.ok(s.end > s.start, `${s.name} is empty`);
    if (i) assert.equal(s.start, SHOTS[i - 1].end, `${s.name} starts where ${SHOTS[i - 1].name} ends`);
  });
  assert.equal(SHOTS[SHOTS.length - 1].end, SONG.duration);
  assert.equal(shotAt(0).name, "title");
  assert.equal(shotAt(SONG.duration).name, "outro");
});

test("every line but the one the end card types is captioned", () => {
  const hidden = Object.entries(PLACE).filter(([, p]) => "hidden" in p && p.hidden);
  assert.deepEqual(hidden.map(([id]) => id), ["outro.3"]);
  assert.deepEqual(
    TIMED.map((l) => l.id),
    LYRICS.map((l) => l.id).filter((id) => id !== "outro.3"),
  );
});

test("a call lands before it is sung and leaves only once it has been", () => {
  for (const l of TIMED) {
    if (!l.call) continue;
    const sung = lyric(l.id).sung;
    const words = l.answer ? sung.slice(0, sung.length - l.answer.lands.length) : sung;
    assert.ok(l.call.land <= words[0][1] - LEAD + 1e-9, `${l.id} lands late`);
    const last = words[words.length - 1][1];
    assert.ok(l.call.out >= last + 0.2, `${l.id} starts to leave ${(l.call.out - last).toFixed(2)} s after its last word is sung`);
  }
});

test("an answer lands each word as it is sung, and holds after the last", () => {
  for (const l of TIMED) {
    if (!l.answer) continue;
    const sung = lyric(l.id).sung;
    assert.deepEqual(l.answer.lands, sung.slice(sung.length - l.answer.lands.length).map(([, t]) => t), l.id);
    assert.ok(l.answer.out >= l.answer.lands[l.answer.lands.length - 1] + ANSWER_HOLD - 1e-9, l.id);
  }
});

test("on each row, a line starts to land only as the one before it starts to leave", () => {
  for (const row of ["call", "answer"] as const) {
    const rows = TIMED.flatMap((l) => {
      const r = l[row];
      return r ? [{ id: l.id, y: r.y, out: r.out, lands: "land" in r ? entrance(r.text, r.land) : r.lands[0] - WORD }] : [];
    });
    for (let i = 1; i < rows.length; i++) {
      const [prev, cur] = [rows[i - 1], rows[i]];
      if (cur.y !== prev.y) continue;
      assert.ok(cur.lands >= prev.out - 1e-9, `${cur.id}'s ${row} starts before ${prev.id}'s leaves`);
    }
  }
});

test("each line is up from its first word's rise until its wipe ends", () => {
  for (const l of TIMED) {
    assert.ok(lyricStart(l) < lyricEnd(l), l.id);
    assert.ok(lyricEnd(l) <= SONG.duration + WIPE, l.id);
  }
  // Placing is a pure function: the same lines come out the same.
  assert.deepEqual(timeLyrics(LYRICS, PLACE), TIMED);
});
