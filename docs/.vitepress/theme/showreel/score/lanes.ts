// Lanes: the seven steps run as bars on four file lanes. The full groove,
// Dm | C | Dm | G, with the tambourine. Bar 2 breaks at b6.5 into muted
// ghost jingles over a bass pedal on A, the clamp on b8 brings the stomp
// back, and over the G the concertina resolves motif M on D, which lifts
// into restore.
//
// Each lane has a pitch: README.md and src/app.ts sound together as D4 + F4
// (one prettier job), src/main.py is A4 and scripts/deploy.sh C5, so the
// four lanes at once are the Dm7 the reel lives in. The cues come from the
// kit's own schedule (kit/lanes.ts) and the scene's beat map
// (scenes/lanes.ts BEATS), so they land where the chart moves: every
// padlock clicks shut when its lock is taken and springs open when it is
// let go, each single-lane step's ✔ rings its lane an octave up, the
// docked steps rattle their locks while they wait, and each step that
// holds every file is the big hit: it winds up out of a breath of quiet,
// dives on its own boot, the crew's hands and a Dm stab with a 32nd run up
// the four lanes, and slams down across them on a stamp, a knock and a
// low thump. The fiddle chops the off-beats around the break.

import type { Part } from ".";
import { CASCADE, chipWidth, DOCK_CHIP, ganttAt, LANES, lanesOf, lockedFrom, SCHEDULE, type ScheduleStep } from "../kit/lanes";
import { BEATS } from "../scenes/lanes";
import { BEAT, type Section } from "../timeline";
import { C, CHOP, CHORD, chopBars, chordBars, DM, type Drums, drumBars, G, HOME, melody, STOMP_FULL } from "./grooves";
import { ad, hz, line, type Mix, swell, sweep, X } from "./mix";
import { bassBar, bassRun, blip, concertina, fiddlePluck, gangClap, jingle, knock, padlock, panX, ping, puff, shimmer, stamp, stomp, thump, tick, whoosh } from "./sounds";

/**
 * Bar 2: the stomp until b6.5, then only the tambourine, muted, on the
 * sixteenths, and nothing on the last one: the clamp lands out of a breath.
 */
const BREAK: Drums = [[0, 8], [4], [2, 6], [10, 11, 12, 13, 14]];
/** Bar 3: the full stomp under the clamps, whose own boots land on 1 and 3. */
const CLAMPED: Drums = [[10], STOMP_FULL[1], STOMP_FULL[2], STOMP_FULL[3]];

/** Each lane's pitch: D4, F4, A4, C5. */
const LANE_NOTE = [62, 65, 69, 72];
/** The second step to hold every file runs a step higher: E4, G4, B4, D5. */
const HIGHER = [64, 67, 71, 74];
/** The clamps' stabs, both over the Dm: D5 F5 A5, then F5 A5 D6. */
const CLAMP_CHORD = [74, 77, 81];
const CLAMP_CHORD_HIGH = [77, 81, 86];
/** Where the padlocks sit. */
const LOCK = panX(LANES.lockX);
/** The steps that wait in the dock, and the centres of their chips there (the kit's dock slots). */
const DOCK_CHIPS = ganttAt(2).chips.filter((c) => c.dock);
const DOCK_STEPS = DOCK_CHIPS.map((c) => c.step);
const DOCKED = DOCK_CHIPS.map((c) => c.x + chipWidth(c.step.step, DOCK_CHIP) / 2);
/** A light crosses the finished chart (scenes/lanes.ts GLINT). */
const GLINT = { from: 13, to: 14.25 } as const;

/** Every padlock event: [lanes-local beat, lane, shut]. A lock handed straight on opens for a sixteenth first. */
function lockEvents(): [number, number, boolean][] {
  const out: [number, number, boolean][] = [];
  for (let lane = 0; lane < LANES.rows.length; lane++) {
    for (const s of SCHEDULE) {
      if (lane < s.lanes[0] || lane > s.lanes[1]) continue;
      out.push([lockedFrom(s), lane, true], [s.end, lane, false]);
    }
  }
  return out;
}

/** A step that holds every file, dropping out of the dock: a clamp. */
const holdsAll = (s: ScheduleStep): boolean => lanesOf(s).length === LANES.rows.length;

/**
 * A step that holds every file leaves the dock: the big hit. It winds up
 * over the sixteenth before, the air cut off a 32nd short of it so the hit
 * lands out of a breath, and dives on its start: a boot on top of the
 * groove's, the crew's hands, a low thump, and a short Dm stab that carries
 * it on a small speaker, while the music ducks hard and a 32nd run climbs
 * the lanes. It lands across all four lanes a sixteenth later (the slam,
 * when the padlocks shut) on a stamp, a wooden knock on D3 and a low thump.
 */
function clamp(m: Mix, s: Section, step: ScheduleStep, slam: number, notes: readonly number[], chord: readonly number[]): void {
  const t = s.beat(step.start);
  const land = s.beat(slam);
  const pan = panX(step.x0);
  whoosh(m, ad(t - X, t - X / 2 - 0.01, 0.03, t - X / 2), sweep(t - X, 900, t - X / 2, 2000), 2, { pan, send: 0.15, hold: false });
  // The dive's air follows the bar down and is gone as it lands.
  whoosh(m, ad(t, t + X / 2, 0.045, land - 0.005), sweep(t, 3200, land, 900), 1.4, { pan, send: 0.15, hold: false });
  m.duck(t, 0.6, 0.2);
  stomp(m, t, 1.2, pan * 0.5);
  gangClap(m, t, 0.95, "sfx");
  jingle(m, t, 1, 0.3, pan);
  thump(m, t, 0.5, 110, 42, 0.3, { pan });
  knock(m, t, hz(62), 0.42, pan, 0.14);
  // The stab is let go before the landing, so the slam speaks.
  concertina(m, t, t + 0.07, chord, 0.17, { attack: 0.006, sustain: 0.55, release: 0.05, bright: 3800, pan, send: 0.2, bus: "sfx" });
  m.duck(land, 0.3, 0.15);
  stamp(m, land, 0.9, pan);
  knock(m, land, hz(62), 0.4, pan, 0.14);
  thump(m, land, 0.55, 110, 42, 0.4, { pan });
  notes.forEach((n, i) => fiddlePluck(m, t + (i * X) / 2 + 0.002, hz(n), 0.07, pan + 0.08 * (i - 1.5), 0.25, { bus: "sfx" }));
}

export const part: Part = {
  // The heart of the reel.
  level: 1.18,
  cues(m, s) {
    // The queue: the two dock chips slide in from the right as a train, a
    // 32nd apart, each with a small closed padlock that rattles as it settles.
    const q = BEATS.queue;
    whoosh(m, ad(s.beat(q), s.beat(q + 0.3), 0.06, s.beat(q + 0.55)), sweep(s.beat(q), 2800, s.beat(q + 0.5), 1200), 1.5, { pan: line(s.beat(q), 0.7, s.beat(q + 0.5), panX(1420)), send: 0.2 });
    DOCKED.forEach((x, k) => padlock(m, s.beat(q + k / 8 + 0.3), true, 0.16 - 0.03 * k, panX(x)));

    // b1, go: prettier, ruff and shfmt start together on different files,
    // three plucks at once (the dyad for prettier's two files).
    m.duck(s.beat(BEATS.go), 0.15, 0.12);
    const go = SCHEDULE.filter((st) => st.start === SCHEDULE[0].start);
    for (const st of go) for (const lane of lanesOf(st)) fiddlePluck(m, s.beat(st.start) + 0.003 * lane, hz(LANE_NOTE[lane]), 0.065, panX(st.x0), 0.22, { bus: "sfx" });

    // The padlocks: shut as a step takes its lock, open as it lets go. Four
    // at once are staggered a few milliseconds, so they sound as four.
    for (const [b, lane, shut] of lockEvents()) {
      padlock(m, s.beat(b) + 0.004 * lane, shut, shut ? 0.3 : 0.2, LOCK + 0.03 * (lane - 1.5));
    }

    // A single-lane step's ✔ rings its lane an octave up.
    for (const st of SCHEDULE) {
      if (lanesOf(st).length > 1) continue;
      ping(m, s.beat(st.end) + 0.01, hz(LANE_NOTE[st.lanes[0]] + 12), 0.075, 0.45, { pan: panX(st.x1), send: 0.3 });
    }

    // A turn on deploy.sh: shfmt hands its lock to shellcheck with a
    // wooden knock on G4, and the key clicks as it hops across.
    knock(m, s.beat(BEATS.shfmtDone), hz(67), 0.3, panX(804), 0.14);
    tick(m, s.beat(BEATS.shfmtDone) + 0.01, 2200, 0.3, panX(804), 0.08);

    // depends: ruff's ✔ sends a spark along the bracket, which snaps taut
    // as it lands, and ruff-format starts on A4.
    whoosh(m, ad(s.beat(BEATS.dependsSpark), s.beat(BEATS.depends) - 0.02, 0.035, s.beat(BEATS.depends) + 0.01), sweep(s.beat(BEATS.dependsSpark), 2000, s.beat(BEATS.depends), 6000), 4, { pan: line(s.beat(BEATS.dependsSpark), panX(880), s.beat(BEATS.depends), panX(908)), send: 0.2, hold: false }, "white");
    blip(m, s.beat(BEATS.depends), hz(81), 0.11);
    fiddlePluck(m, s.beat(BEATS.depends) + 0.004, hz(LANE_NOTE[2]), 0.07, panX(908), 0.22, { bus: "sfx", bright: 9 });

    // The docked steps knock on the locks they wait for, each knock a
    // rattle of its small padlock; the air draws in toward the clamp.
    for (const at of BEATS.knocks) {
      DOCK_STEPS.forEach((st, k) => {
        if (at < st.start) padlock(m, s.beat(at) + 0.03 * k, true, 0.16 - 0.02 * k, panX(DOCKED[k]));
      });
    }
    const clamps = SCHEDULE.filter(holdsAll);
    const first = s.beat(clamps[0].start);
    // It is sucked away a sixteenth before the clamp, which lands out of the quiet.
    const suck = first - X;
    whoosh(m, swell(first - 1.25 * BEAT, first - BEAT, 0.01, suck - 0.03, 0.05, suck), sweep(first - 1.25 * BEAT, 1500, suck, 6000), 1.1, { send: 0.15 }, "white");

    // The clamps: every file at once on a Dm stab, then again a step higher.
    clamps.forEach((st, i) => clamp(m, s, st, i ? BEATS.slam2 : BEATS.slam, i ? HIGHER : LANE_NOTE, i ? CLAMP_CHORD_HIGH : CLAMP_CHORD));
    // The files the clamp was waiting on flash warm.
    puff(m, s.beat(BEATS.clamp) + 0.01, 0.03, panX(1320), false);

    // b12: the ✔ cascade down the lanes, D5 F5 A5 D6, one a sixteenth.
    [74, 77, 81, 86].forEach((n, lane) => {
      ping(m, s.beat(CASCADE.beat + lane * CASCADE.each), hz(n), 0.1, 0.7, { pan: panX(CASCADE.x), send: 0.35 });
    });

    // A light crosses the finished chart.
    shimmer(m, s.beat(GLINT.from), s.beat(GLINT.to), 0.018, 0);
    // The note and the dock clear away.
    const out = s.beat(BEATS.detailOut);
    whoosh(m, ad(out, out + 0.15, 0.045, out + 0.35), sweep(out, 1200, out + 0.35, 3000), 1.2, { pan: line(out, -0.2, out + 0.35, 0.4), send: 0.2 });
  },
  // The heart of the reel: the crew's hands at full strength. In bar 3 the
  // clamps bring their own boots on 1 and 3, so the groove leaves those out.
  drums: (m, s) => drumBars(m, s, [STOMP_FULL, BREAK, CLAMPED, STOMP_FULL], 1),
  bass(m, s) {
    bassBar(m, s.bar(0), ...DM);
    // Bar 2 bounces on C until the break, then holds A, the dominant, into the clamp's Dm.
    bassBar(m, s.bar(1), ...C, [
      [0, 3, 0, 1],
      [4, 3, 7, 0.8],
      [8, 2, 0, 0.9],
    ]);
    bassRun(m, [[s.beat(6.5), s.beat(8) - 0.02, 33, 0.9]], 0.045, 300, 0.75);
    bassBar(m, s.bar(2), ...DM);
    bassBar(m, s.bar(3), ...G);
  },
  lead: (m, s) => melody(m, s.beat(12), HOME),
  pads(m, s) {
    const chords = [CHORD.Dm, CHORD.C, CHORD.Dm, CHORD.G];
    chordBars(m, s, chords);
    // The fiddle chops the off-beats, all but the break's, harder than in
    // the commit: the heart of the reel.
    chopBars(m, s, chords, 1.5 * CHOP, 0, 1);
    chopBars(m, s, chords, 1.5 * CHOP, 1, 2, [1.5]);
    chopBars(m, s, chords, 1.5 * CHOP, 2, 4);
  },
};
