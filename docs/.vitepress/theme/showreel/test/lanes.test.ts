// The lanes play one real pre-commit run (storyboard K3): canon80's
// commit.frames.txt. The order of the bars comes from hk's header count, a
// lane never holds two steps at once, a lock handed straight on leaves an
// 8 px gap, and the bars grow with one playhead to the ends the storyboard
// gives. The finished chart is the lanes|restore handoff and the empty one
// the stash|lanes handoff.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { HKPKL_STEPS } from "../kit/card";
import {
  chipWidth,
  DOCK,
  GANTT_SETTLED,
  ganttAt,
  HANDOFF_GAP,
  LANE_FILES,
  LANES,
  PLAYHEAD,
  playheadX,
  SCHEDULE,
  type ScheduleStep,
} from "../kit/lanes";
import { SHOWREEL } from "./repo";

const onLane = (lane: number) => SCHEDULE.filter((s) => lane >= s.lanes[0] && lane <= s.lanes[1]).sort((a, b) => a.start - b.start);
const sampleBeats = Array.from({ length: 16 * 16 + 1 }, (_, i) => i / 16);

test("SCHEDULE is the storyboard's table", () => {
  const row = (s: ScheduleStep) => `${s.step} ${s.lanes.join("-")} b${s.start} ${s.x0} → b${s.end} ${s.x1}`;
  assert.deepEqual(SCHEDULE.map(row), [
    "prettier 0-1 b1 648 → b8 1320",
    "ruff 2-2 b1 648 → b3.5 880",
    "shfmt 3-3 b1 648 → b2.5 800",
    "shellcheck 3-3 b2.5 808 → b5 1100",
    "ruff-format 2-2 b4 908 → b6.5 1230",
    "trailing-whitespace 0-3 b8 1328 → b10 1500",
    "newlines 0-3 b10 1508 → b12 1690",
  ]);
  assert.deepEqual(LANE_FILES, ["README.md", "src/app.ts", "src/main.py", "scripts/deploy.sh"]);
  assert.deepEqual(LANES.rows, [200, 320, 440, 560]);
  // The seven steps of the hk.pkl the config scene shows, no more.
  assert.deepEqual(SCHEDULE.map((s) => s.step).sort(), HKPKL_STEPS.map((s) => s.step).sort());
});

test("the playhead's keyframes run forward, and every bar starts and ends on it", () => {
  for (let i = 1; i < PLAYHEAD.length; i++) {
    assert.ok(PLAYHEAD[i][0] > PLAYHEAD[i - 1][0], `beat ${PLAYHEAD[i][0]}`);
    assert.ok(PLAYHEAD[i][1] > PLAYHEAD[i - 1][1], `x ${PLAYHEAD[i][1]}`);
  }
  for (const [b, x] of PLAYHEAD) assert.equal(playheadX(b), x);
  assert.equal(playheadX(0), 648);
  assert.equal(playheadX(16), 1690);
  for (let b = 0; b < 16; b += 1 / 16) assert.ok(playheadX(b + 1 / 16) >= playheadX(b));
  for (const s of SCHEDULE) {
    assert.equal(s.x1, playheadX(s.end), `${s.step} ends on the playhead`);
    const handed = SCHEDULE.some((o) => o.end === s.start && o.lanes[0] <= s.lanes[1] && o.lanes[1] >= s.lanes[0]);
    assert.equal(s.x0, playheadX(s.start) + (handed ? HANDOFF_GAP : 0), `${s.step} starts on the playhead${handed ? " plus the hand-off gap" : ""}`);
    assert.ok(s.start < s.end && s.x0 < s.x1, s.step);
    // Inside the track, 8 px in at the left.
    assert.ok(s.x0 >= LANES.trackX0 + 8 && s.x1 <= LANES.trackX1, s.step);
  }
});

test("a lane holds one step at a time, and a lock handed straight on leaves an 8 px gap", () => {
  for (let lane = 0; lane < 4; lane++) {
    const steps = onLane(lane);
    for (let i = 1; i < steps.length; i++) {
      const [a, b] = [steps[i - 1], steps[i]];
      assert.ok(b.start >= a.end, `${LANE_FILES[lane]}: ${b.step} starts after ${a.step} ends`);
      if (b.start === a.end) assert.equal(b.x0 - a.x1, HANDOFF_GAP, `${LANE_FILES[lane]}: ${a.step} hands its lock to ${b.step}`);
      else assert.ok(b.x0 - a.x1 > HANDOFF_GAP, `${LANE_FILES[lane]}: ${a.step} → ${b.step} is no hand-off`);
    }
    for (const b of sampleBeats) {
      const running = steps.filter((s) => b >= s.start && b < s.end);
      assert.ok(running.length <= 1, `${LANE_FILES[lane]} at b${b}: ${running.map((s) => s.step)}`);
    }
  }
  // The hand-offs the lanes scene stages: shfmt → shellcheck, prettier → trailing-whitespace, trailing-whitespace → newlines.
  const handoffs = SCHEDULE.flatMap((b) => SCHEDULE.filter((a) => a.end === b.start && a.lanes[0] <= b.lanes[1] && a.lanes[1] >= b.lanes[0]).map((a) => `${a.step}→${b.step}`));
  assert.deepEqual(handoffs.sort(), ["prettier→trailing-whitespace", "shfmt→shellcheck", "trailing-whitespace→newlines"]);
});

test("the bars end in the order of hk's header count, frame by frame in canon80", () => {
  const byEnd = [...SCHEDULE].sort((a, b) => a.end - b.end);
  assert.deepEqual(
    byEnd.map((s) => s.done),
    [1, 2, 3, 4, 5, 6, 7],
  );
  const frames = readFileSync(join(SHOWREEL, "test/captures/commit.frames.txt"), "utf8")
    .split(/^----- frame \d+ -----\n/m)
    .slice(1)
    .map((f) => f.replace(/\n+$/, "").split("\n"));
  const count = (f: number) => Number(/\] (\d+)\/7$/.exec(frames[f][0])?.[1]);
  const started = (f: number, step: string) => frames[f].includes(`❯ ${step}`) || frames[f].some((l) => l.startsWith(`✔ ${step} `) || l === `✔ ${step}`);
  for (const s of SCHEDULE) {
    assert.equal(count(s.frames.end), s.done, `${s.step}: frame ${s.frames.end} shows ${s.done}/7`);
    assert.equal(count(s.frames.end - 1), s.done - 1, `${s.step}: frame ${s.frames.end - 1} shows ${s.done - 1}/7`);
    assert.ok(started(s.frames.start, s.step), `${s.step}: its row appears in frame ${s.frames.start}`);
    assert.ok(!started(s.frames.start - 1, s.step), `${s.step}: and not before`);
    assert.ok(s.frames.start < s.frames.end, s.step);
  }
  // Steps that start together on the board start in the same frame.
  for (const a of SCHEDULE) for (const b of SCHEDULE) if (a.start === b.start) assert.equal(a.frames.start, b.frames.start, `${a.step} and ${b.step}`);
});

test("padlocks: shut while a step holds the lane, open for a sixteenth at a hand-off", () => {
  const states = (b: number) => ganttAt(b).locks.map((l) => l.state).join(" ");
  assert.equal(states(0), "open open open open");
  assert.equal(states(1.5), "write write write write");
  assert.equal(states(2.6), "write write write open", "shfmt hands deploy.sh to shellcheck");
  assert.equal(states(2.8), "write write write write");
  assert.equal(states(3.75), "write write open write", "ruff done, ruff-format waits on depends");
  assert.equal(states(4.1), "write write write write");
  assert.equal(states(5.5), "write write write open");
  assert.equal(states(7), "write write open open", "only prettier");
  assert.equal(states(8.1), "open open open open", "prettier hands on to trailing-whitespace");
  assert.equal(states(8.3), "write write write write", "trailing-whitespace takes all four at once");
  assert.equal(states(10.1), "open open open open");
  assert.equal(states(10.3), "write write write write");
  assert.equal(states(12.1), "open open open open");
  // Shut exactly while some step holds the lane.
  for (const b of sampleBeats) {
    ganttAt(b).locks.forEach((l, lane) => {
      const held = onLane(lane).some((s) => b >= s.start && b < s.end);
      if (l.state === "write") assert.ok(held, `${LANE_FILES[lane]} locked at b${b} with no step`);
      if (!held) assert.equal(l.state, "open", `${LANE_FILES[lane]} at b${b}`);
    });
  }
});

test("waiting chips ride ahead of the playhead, and the dock's queue fits above the lanes", () => {
  for (const b of sampleBeats) {
    const g = ganttAt(b);
    for (const c of g.chips) {
      assert.ok(c.x >= c.step.x0 || c.dock, `${c.step.step} at b${b} is at or past its slot`);
      if (!c.dock && b >= 1 && b < c.step.start) assert.ok(c.x >= playheadX(b) + 12 - 1e-9, `${c.step.step} ahead of the playhead at b${b}`);
      assert.ok(b < c.step.start + 0.25, `${c.step.step} gone once it runs`);
    }
    const dock = g.chips.filter((c) => c.dock).sort((a, b) => a.x - b.x);
    for (let i = 1; i < dock.length; i++) {
      assert.ok(dock[i - 1].x + chipWidth(dock[i - 1].step.step, { size: 28, lock: true }) + HANDOFF_GAP <= dock[i].x + 1e-9, `dock chips overlap at b${b}`);
    }
    for (const c of dock) {
      assert.ok(c.x + chipWidth(c.step.step, { size: 28, lock: true }) <= LANES.trackX1, `${c.step.step} inside the margin`);
      assert.ok(b < 1.5 || Math.abs(c.cy - (DOCK.y0 + DOCK.y1) / 2) < 1e-9, `${c.step.step} settled in the dock`);
    }
  }
  // The steps that wait are exactly those that start after the first downbeat.
  const waiting = new Set(sampleBeats.flatMap((b) => ganttAt(b).chips.map((c) => c.step.step)));
  assert.deepEqual([...waiting].sort(), ["newlines", "ruff-format", "shellcheck", "trailing-whitespace"]);
});

test("the chart starts as the empty lanes and settles into the finished Gantt", () => {
  const start = ganttAt(0);
  assert.equal(start.bars.length, 0);
  assert.equal(start.playhead.alpha, 0);
  assert.ok(start.chips.every((c) => c.alpha === 0));
  assert.ok(start.checks.every((k) => k === 0));
  assert.ok(start.locks.every((l) => l.state === "open" && l.lift === 1), "open padlocks, as drawLanes draws them");

  const end = ganttAt(16);
  assert.equal(end.bars.length, SCHEDULE.length);
  for (const b of end.bars) assert.equal(b.x1, b.step.x1, `${b.step.step} at full extent`);
  assert.equal(end.playhead.alpha, 0);
  assert.equal(end.chips.length, 0);
  assert.deepEqual(end.checks, [1, 1, 1, 1]);
  assert.ok(end.locks.every((l) => l.state === "open" && l.lift === 1));
  for (let b = GANTT_SETTLED; b <= 16; b += 1 / 8) {
    // Done badges gone, the cascade landed, the playhead faded: nothing moves.
    assert.deepEqual(ganttAt(b), end, `still at b${b}`);
  }
});

test("frames are a pure function of the beat", () => {
  for (const b of [0.6, 2.55, 4.2, 8.05, 10.2, 12.3]) assert.deepEqual(ganttAt(b), ganttAt(b));
});
