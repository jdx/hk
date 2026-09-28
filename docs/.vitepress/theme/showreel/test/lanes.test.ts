// The lanes play one real pre-commit run (storyboard K3): the commit
// capture's commit.frames.txt, with the order of its steps' patches and
// hand-offs from commit.locks.txt. A step holds its files in read and write phases; a lane
// holds one writer or any number of readers, a lock handed straight on
// leaves an 8 px gap, and the bars grow with one playhead to the ends the
// storyboard gives. The order of the bars' ends comes from hk's header
// count. The finished chart is the lanes|restore handoff and the empty one
// the stash|lanes handoff.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { HKPKL_STEPS } from "../kit/card";
import {
  CASCADE,
  chipWidth,
  DOCK,
  GANTT_SETTLED,
  ganttAt,
  HANDOFF_GAP,
  holdsOn,
  LANE_FILES,
  LANES,
  lanesOf,
  lockChanges,
  lockedFrom,
  type Phase,
  phaseLockedFrom,
  PLAYHEAD,
  playheadX,
  SCHEDULE,
  type ScheduleStep,
  stepSpan,
  WROTE,
} from "../kit/lanes";
import { BEATS } from "../scenes/lanes-timing";
import { SHOWREEL } from "./repo";

const sampleBeats = Array.from({ length: 16 * 16 + 1 }, (_, i) => i / 16);
const step = (name: string): ScheduleStep => SCHEDULE.find((s) => s.step === name)!;
const shares = (a: ScheduleStep, b: ScheduleStep) => a.lanes[0] <= b.lanes[1] && b.lanes[0] <= a.lanes[1];
/** Every phase of every step, with its step and index. */
const PHASES = SCHEDULE.flatMap((s) => s.phases.map((p, i) => ({ s, p, i })));
/** Phases on a shared lane that can't hold it at once: a writer shares with nobody. */
const clash = (p: Phase, q: Phase) => p.lock === "write" || q.lock === "write";
/** The phase that hands its file straight to phase `i` of `s`: it ends as this starts, and they can't share it. */
const handedBy = (s: ScheduleStep, i: number) => PHASES.find((o) => shares(o.s, s) && !(o.s === s && o.i === i) && o.p.end === s.phases[i].start && clash(o.p, s.phases[i]));

const frames = readFileSync(join(SHOWREEL, "test/captures/commit.frames.txt"), "utf8")
  .split(/^----- frame \d+ -----\n/m)
  .slice(1)
  .map((f) => f.replace(/\n+$/, "").split("\n"));
const running = (f: number, name: string) => frames[f].includes(`❯ ${name}`);

test("SCHEDULE is the storyboard's table", () => {
  const phase = (p: Phase) => `${p.lock} b${p.start} ${p.x0} → b${p.end} ${p.x1}`;
  const row = (s: ScheduleStep) => [s.step, s.lanes.join("-"), s.half === undefined ? "" : ["top", "bottom"][s.half], s.twin ? `twin ${s.twin}` : "", s.phases.map(phase).join(", ")].filter(Boolean).join(" ");
  assert.deepEqual(SCHEDULE.map(row), [
    "prettier 0-1 write b1 648 → b10 1500",
    "ruff 2-2 write b1 648 → b4.5 980",
    "ruff-format 2-2 read b5 1027 → b7.5 1264, write b7.5 1272 → b8.5 1358",
    "shfmt 3-3 top read b1 648 → b2 743, write b3 846 → b4 932",
    "shellcheck 3-3 bottom read b1 648 → b3 838",
    "trailing-whitespace 0-3 twin newlines read b10 1508 → b12 1690",
    "newlines 0-3 twin trailing-whitespace read b10 1508 → b12 1690",
  ]);
  assert.deepEqual(LANE_FILES, ["README.md", "src/app.ts", "src/main.py", "scripts/deploy.sh"]);
  assert.deepEqual(LANES.rows, [200, 320, 440, 560]);
  // The seven steps of the hk.pkl the config scene shows, no more.
  assert.deepEqual(SCHEDULE.map((s) => s.step).sort(), HKPKL_STEPS.map((s) => s.step).sort());
  for (const s of SCHEDULE) {
    const [first, last] = [s.phases[0], s.phases[s.phases.length - 1]];
    assert.deepEqual([s.start, s.x0, s.end, s.x1], [first.start, first.x0, last.end, last.x1], `${s.step}'s span is its phases'`);
    for (let i = 1; i < s.phases.length; i++) assert.ok(s.phases[i].start >= s.phases[i - 1].end && s.phases[i].x0 > s.phases[i - 1].x1, `${s.step}'s phases in order`);
  }
  // Twins are one bar: the same lanes and phases, each naming the other.
  for (const s of SCHEDULE.filter((s) => s.twin)) {
    const t = step(s.twin!);
    assert.equal(t.twin, s.step);
    assert.deepEqual([t.lanes, t.phases], [s.lanes, s.phases], `${s.step} and ${t.step}`);
  }
});

test("the playhead's keyframes run forward, are the phases' ends, and every bar starts and ends on it", () => {
  for (let i = 1; i < PLAYHEAD.length; i++) {
    assert.ok(PLAYHEAD[i][0] > PLAYHEAD[i - 1][0], `beat ${PLAYHEAD[i][0]}`);
    assert.ok(PLAYHEAD[i][1] > PLAYHEAD[i - 1][1], `x ${PLAYHEAD[i][1]}`);
  }
  for (const [b, x] of PLAYHEAD) assert.equal(playheadX(b), x);
  assert.equal(playheadX(0), 648);
  assert.equal(playheadX(16), 1690);
  for (let b = 0; b < 16; b += 1 / 16) assert.ok(playheadX(b + 1 / 16) >= playheadX(b));
  // A keyframe at each beat a phase starts or ends, and nowhere else.
  const ends = [...new Set(PHASES.flatMap(({ p }) => [p.start, p.end]))].sort((a, b) => a - b);
  assert.deepEqual(
    PLAYHEAD.map(([b]) => b),
    ends,
  );
  for (const { s, p, i } of PHASES) {
    const name = `${s.step} ${p.lock}`;
    assert.equal(p.x1, playheadX(p.end), `${name} ends on the playhead`);
    const handed = handedBy(s, i) !== undefined;
    assert.equal(p.x0, playheadX(p.start) + (handed ? HANDOFF_GAP : 0), `${name} starts on the playhead${handed ? " plus the hand-off gap" : ""}`);
    assert.ok(p.start < p.end && p.x0 < p.x1, name);
    // Inside the track, 8 px in at the left.
    assert.ok(p.x0 >= LANES.trackX0 + 8 && p.x1 <= LANES.trackX1, name);
  }
});

test("a lane holds one writer or any number of readers, and a lock handed straight on leaves an 8 px gap", () => {
  for (let lane = 0; lane < 4; lane++) {
    const holds = holdsOn(lane);
    for (const b of sampleBeats) {
      const now = holds.filter((h) => b >= h.from && b < h.to);
      if (now.some((h) => h.lock === "write")) assert.equal(now.length, 1, `${LANE_FILES[lane]} at b${b}: ${now.map((h) => h.step.step)}`);
    }
  }
  // Phases that can't share a file never overlap on it, in beats or in px;
  // straight on, they are the gap apart.
  for (const a of PHASES) {
    for (const b of PHASES) {
      if (a === b || !shares(a.s, b.s) || !clash(a.p, b.p) || a.p.start > b.p.start || (a.p.start === b.p.start && a.p.end > b.p.end)) continue;
      const name = `${a.s.step} ${a.p.lock} → ${b.s.step} ${b.p.lock}`;
      assert.ok(b.p.start >= a.p.end, `${name}: one after the other`);
      if (b.p.start === a.p.end) assert.equal(b.p.x0 - a.p.x1, HANDOFF_GAP, `${name}: handed straight on`);
      else assert.ok(b.p.x0 - a.p.x1 > HANDOFF_GAP, `${name}: no hand-off`);
    }
  }
  // Readers that share a lane at once each take half its band, or are twins drawn as one bar.
  for (const a of PHASES) {
    for (const b of PHASES) {
      if (a.s === b.s || !shares(a.s, b.s) || a.p.lock !== "read" || b.p.lock !== "read" || a.p.end <= b.p.start || b.p.end <= a.p.start) continue;
      if (a.s.twin === b.s.step) continue;
      assert.ok(a.s.half !== undefined && b.s.half !== undefined && a.s.half !== b.s.half, `${a.s.step} and ${b.s.step} read in halves`);
      const [sa, sb] = [stepSpan(a.s), stepSpan(b.s)];
      assert.ok(sa.y1 <= sb.y0 || sb.y1 <= sa.y0, `${a.s.step} and ${b.s.step} don't overlap`);
    }
  }
  // A half is only for a reader that shares its lane.
  for (const s of SCHEDULE.filter((s) => s.half !== undefined)) {
    assert.ok(
      SCHEDULE.some((o) => o !== s && shares(o, s) && o.half !== undefined && o.half !== s.half && o.start < s.end && s.start < o.end),
      `${s.step} shares its lane`,
    );
  }
  // The hand-offs the lanes scene stages: prettier's files to the two whole-repo readers,
  // deploy.sh from shellcheck to shfmt's patch, and main.py from ruff-format's read to its write.
  const handoffs = PHASES.flatMap(({ s, p, i }) => {
    const o = handedBy(s, i);
    return o ? [`${o.s.step} ${o.p.lock} → ${s.step} ${p.lock}`] : [];
  });
  assert.deepEqual(handoffs.sort(), ["prettier write → newlines read", "prettier write → trailing-whitespace read", "ruff-format read → ruff-format write", "shellcheck read → shfmt write"]);
});

test("the bars end in the order of hk's header count, frame by frame in the capture", () => {
  const byEnd = [...SCHEDULE].sort((a, b) => a.end - b.end);
  assert.deepEqual(
    byEnd.map((s) => s.done),
    [1, 2, 3, 4, 5, 6, 7],
  );
  const count = (f: number) => Number(/\] (\d+)\/7$/.exec(frames[f][0])?.[1]);
  const started = (f: number, name: string) => running(f, name) || frames[f].some((l) => l.startsWith(`✔ ${name} `) || l === `✔ ${name}`);
  for (const s of SCHEDULE) {
    assert.equal(count(s.frames.end), s.done, `${s.step}: frame ${s.frames.end} shows ${s.done}/7`);
    assert.equal(count(s.frames.end - 1), s.done - 1, `${s.step}: frame ${s.frames.end - 1} shows ${s.done - 1}/7`);
    assert.ok(started(s.frames.start, s.step), `${s.step}: its row appears in frame ${s.frames.start}`);
    assert.ok(!started(s.frames.start - 1, s.step), `${s.step}: and not before`);
    assert.ok(s.frames.start < s.frames.end, s.step);
  }
  // Steps start together on the board exactly when they start in the same frame.
  for (const a of SCHEDULE) for (const b of SCHEDULE) assert.equal(a.start === b.start, a.frames.start === b.frames.start, `${a.step} and ${b.step}`);
});

test("the steps that share a file at once on the board ran at once in the capture", () => {
  const pair = (a: ScheduleStep, b: ScheduleStep) => [a.step, b.step].sort().join(" + ");
  const board = new Set<string>();
  const capture = new Set<string>();
  for (const a of SCHEDULE) {
    for (const b of SCHEDULE) {
      if (a === b || !shares(a, b)) continue;
      if (a.start < b.end && b.start < a.end) board.add(pair(a, b));
      if (frames.some((_, f) => running(f, a.step) && running(f, b.step))) capture.add(pair(a, b));
    }
  }
  assert.deepEqual([...board].sort(), ["newlines + trailing-whitespace", "shellcheck + shfmt"]);
  assert.deepEqual([...capture].sort(), [...board].sort());
});

test("the run's log orders the steps' patches and hand-offs the board shows", () => {
  const log = readFileSync(join(SHOWREEL, "test/captures/commit.locks.txt"), "utf8").split("\n");
  const at = (line: string) => {
    const i = log.findIndex((l) => l.startsWith(line));
    assert.ok(i >= 0, `commit.locks.txt has ${line}`);
    return i;
  };
  const before = (...lines: string[]) => {
    for (let k = 1; k < lines.length; k++) assert.ok(at(lines[k - 1]) < at(lines[k]), `${lines[k - 1]} before ${lines[k]}`);
  };
  const [shfmt, shellcheck, ruff, ruffFormat, tw, nl] = ["shfmt", "shellcheck", "ruff", "ruff-format", "trailing-whitespace", "newlines"].map(step);
  // shfmt and shellcheck both start reading deploy.sh; shfmt has its patch
  // before shellcheck has read it, and applies it only after.
  before("$ shfmt -d", "shfmt: failed check step first", "shellcheck: check_diff succeeded", "shfmt: applied diff");
  before("$ shellcheck --format=diff", "shfmt: failed check step first");
  assert.ok(shfmt.phases[0].end < shellcheck.end && shellcheck.end === shfmt.phases[1].start);
  // shellcheck's diff was empty: it never writes.
  assert.match(log[at("shellcheck: check_diff succeeded")], /stdout len=0/);
  assert.deepEqual(
    shellcheck.phases.map((p) => p.lock),
    ["read"],
  );
  // ruff-format waits for ruff, reads main.py, then applies its patch.
  before("ruff-format: waiting for ruff", "ruff: applied diff", "$ ruff format", "ruff-format: failed check step first", "ruff-format: applied diff");
  assert.ok(ruff.end < ruffFormat.start);
  assert.deepEqual(
    ruffFormat.phases.map((p) => p.lock),
    ["read", "write"],
  );
  // The two whole-repo steps both start reading before either is done, and find nothing.
  before("$ hk util end-of-file-fixer --diff", "trailing-whitespace: check_diff succeeded");
  before("$ hk util trailing-whitespace --diff", "trailing-whitespace: check_diff succeeded");
  before("$ hk util trailing-whitespace --diff", "newlines: check_diff succeeded");
  for (const name of ["trailing-whitespace", "newlines"]) assert.match(log[at(`${name}: check_diff succeeded`)], /stdout len=0/);
  assert.equal(tw.start, nl.start);
});

test("padlocks: warm for a writer, cyan with a pip per reader, open for a sixteenth at a hand-off", () => {
  const states = (b: number) =>
    ganttAt(b)
      .locks.map((l) => (l.state === "read" ? `read${l.pips}` : l.state))
      .join(" ");
  assert.equal(states(0), "open open open open");
  assert.equal(states(1.5), "write write write read2", "two fixers write their files, two steps read deploy.sh");
  assert.equal(states(2.5), "write write write read1", "shfmt has its patch and lets deploy.sh go");
  assert.equal(states(3.1), "write write write open", "shellcheck done, the key hops to shfmt");
  assert.equal(states(3.5), "write write write write", "three steps write all four files at once");
  assert.equal(states(4.25), "write write write open");
  assert.equal(states(4.75), "write write open open", "ruff done, ruff-format waits on depends");
  assert.equal(states(6), "write write read1 open", "ruff-format reads main.py beside prettier");
  assert.equal(states(7.6), "write write open open", "ruff-format trades its read for a write");
  assert.equal(states(8), "write write write open");
  assert.equal(states(9), "write write open open", "only prettier");
  assert.equal(states(10.1), "open open open open", "prettier hands on to the two whole-repo steps");
  assert.equal(states(11), "read2 read2 read2 read2", "they read all four files together");
  assert.equal(states(12.1), "open open open open");
  // Each padlock is exactly who holds the file.
  for (const b of sampleBeats) {
    ganttAt(b).locks.forEach((l, lane) => {
      const now = holdsOn(lane).filter((h) => b >= h.from && b < h.to);
      const want = now.length === 0 ? "open" : now.some((h) => h.lock === "write") ? "write" : "read";
      assert.equal(l.state, want, `${LANE_FILES[lane]} at b${b}`);
      assert.equal(l.pips, want === "read" ? now.length : 0, `${LANE_FILES[lane]}'s pips at b${b}`);
    });
  }
  // And its changes, which the picture punches and the score clicks.
  const changes = (lane: number) => lockChanges(lane).map((c) => `b${c.at} ${c.change} ${c.state}${c.state === "read" ? c.pips : ""}`);
  const passes = ["b10.25 shut read2", "b12 open open"];
  for (const lane of [0, 1]) assert.deepEqual(changes(lane), ["b1 shut write", "b10 open open", ...passes]);
  assert.deepEqual(changes(2), ["b1 shut write", "b4.5 open open", "b5 shut read1", "b7.5 open open", "b7.75 shut write", "b8.5 open open", ...passes]);
  assert.deepEqual(changes(3), ["b1 shut read2", "b2 pips read1", "b3 open open", "b3.25 shut write", "b4 open open", ...passes]);
});

test("the picture's beats are the schedule's", () => {
  const [prettier, ruff, ruffFormat, shfmt, shellcheck, tw, nl] = SCHEDULE;
  assert.deepEqual(
    SCHEDULE.filter((s) => s.start === BEATS.go).map((s) => s.step),
    ["prettier", "ruff", "shfmt", "shellcheck"],
  );
  assert.equal(BEATS.go, PLAYHEAD[0][0]);
  assert.equal(BEATS.shfmtPatch, shfmt.phases[0].end);
  assert.equal(BEATS.shellcheckDone, shellcheck.end);
  assert.equal(BEATS.shfmtWrite, phaseLockedFrom(shfmt, 1));
  assert.equal(BEATS.shfmtDone, shfmt.end);
  assert.equal(BEATS.ruffDone, ruff.end);
  assert.equal(BEATS.dependsSpark, ruff.end);
  assert.equal(BEATS.depends, ruffFormat.start);
  assert.equal(BEATS.ruffFormatWrite, phaseLockedFrom(ruffFormat, 1));
  assert.equal(BEATS.ruffFormatDone, ruffFormat.end);
  assert.equal(BEATS.clamp, prettier.end);
  assert.equal(BEATS.slam, lockedFrom(tw));
  assert.equal(BEATS.slam, lockedFrom(nl));
  assert.equal(BEATS.passesDone, tw.end);
  assert.equal(BEATS.passesDone, nl.end);
  assert.deepEqual(
    BEATS.cascade,
    LANES.rows.map((_, lane) => CASCADE.beat + lane * CASCADE.each),
  );
  assert.equal(BEATS.glint[0], GANTT_SETTLED, "the light crosses once the chart is still");
  assert.ok(BEATS.queue < BEATS.go && BEATS.glint[1] < BEATS.detailOut);
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
  assert.deepEqual([...waiting].sort(), ["newlines", "ruff-format", "trailing-whitespace"]);
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

test("bars grow with the playhead through each phase whose lock they hold, and wait on a tie between", () => {
  for (const b of sampleBeats) {
    for (const bar of ganttAt(b).bars) {
      const s = bar.step;
      assert.ok(b >= lockedFrom(s), `${s.step} drawn once it has its lock`);
      assert.ok(bar.x1 >= s.x0 && bar.x1 <= s.x1, `${s.step} at b${b}`);
      assert.ok(bar.x1 <= Math.max(playheadX(b), s.x0) + 1e-9, `${s.step} never ahead of the playhead at b${b}`);
      s.phases.forEach((p, i) => {
        if (b >= phaseLockedFrom(s, i) + 1 / 8 && b < p.end) assert.equal(bar.x1, playheadX(b), `${s.step} ${p.lock} on the playhead at b${b}`);
      });
    }
  }
  // shfmt, waiting for deploy.sh, reaches its write's start and holds there until its lock shuts.
  const shfmt = step("shfmt");
  const reach = (b: number) => ganttAt(b).bars.find((x) => x.step === shfmt)!.x1;
  assert.equal(reach(BEATS.shfmtWrite - 1e-9), shfmt.phases[1].x0);
  assert.equal(reach(BEATS.shfmtWrite + 1 / 8), playheadX(BEATS.shfmtWrite + 1 / 8));
});

test("fixers run in parallel: several at once for most of the run, and three writing all four files together", () => {
  const runningAt = (b: number) => SCHEDULE.filter((s) => b >= s.start && b < s.end);
  const writersAt = (b: number) => PHASES.filter(({ s, p, i }) => b >= phaseLockedFrom(s, i) && b < p.end && p.lock === "write").map(({ s }) => s);
  const dt = 1 / 16;
  let together = 0;
  for (let b = 1; b < 12; b += dt) if (runningAt(b).length >= 2) together += dt;
  assert.equal(together, 9, "all but b4.5–b5, ruff-format waiting on ruff, and b8.5–b10, prettier alone");
  assert.equal(runningAt(1.5).length, 4, "four steps start together");
  const three = writersAt(3.5);
  assert.deepEqual(
    three.map((s) => s.step),
    ["prettier", "ruff", "shfmt"],
  );
  assert.equal(new Set(three.flatMap(lanesOf)).size, 4, "on files of their own");
  // Two steps read one file at once, and at the end every file.
  assert.equal(holdsOn(3).filter((h) => h.from <= 1.5 && 1.5 < h.to).length, 2);
  for (let lane = 0; lane < 4; lane++) assert.equal(holdsOn(lane).filter((h) => h.from <= 11 && 11 < h.to).length, 2);
});

test("WROTE is the files each step's ✔ row says it modified, all under its write locks", () => {
  const rows = readFileSync(join(SHOWREEL, "test/captures/commit.screen.txt"), "utf8").split("\n");
  for (const s of SCHEDULE) {
    const row = rows.find((l) => l === `✔ ${s.step}` || l.startsWith(`✔ ${s.step}  – `));
    assert.ok(row, `${s.step} has a ✔ row`);
    const modified = /– \d+ files? modified – (.+)$/.exec(row)?.[1].split(", ") ?? [];
    assert.deepEqual(
      WROTE[s.step].map((lane) => LANE_FILES[lane]),
      modified,
      `${s.step}: ${row}`,
    );
    // A step that only reads writes nothing; one that writes wrote its lanes.
    const writes = s.phases.some((p) => p.lock === "write");
    assert.deepEqual(WROTE[s.step], writes ? lanesOf(s) : [], s.step);
  }
});

test("frames are a pure function of the beat", () => {
  for (const b of [0.6, 2.55, 3.3, 4.2, 7.7, 8.05, 10.2, 12.3]) assert.deepEqual(ganttAt(b), ganttAt(b));
});
