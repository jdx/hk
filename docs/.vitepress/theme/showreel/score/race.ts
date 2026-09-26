// Race: the published benchmark. The peak: the full stomp and the
// tambourine over Dm | C | Dm | G, and the concertina's whole chorus,
// motif M and then its answer, which ends on D over the G and lifts into
// the morph. Bar 4 turns to boots on the eighths at b14, and everything
// drops at b15.5 under the riser (score/morph.ts).
//
// The picture's cues come from the same facts and the same timing module as
// its bars (race-timing.ts), and from the chart's own beats
// (scenes/race-chart.ts), so every ding lands on the frame its bar stops,
// whichever races the facts back. The whip's air comes in from the right
// with the chart. Each race starts with a fiddle sliding up from the
// stubs' crouch, a gun as the bars leave the axis (a boot, the crew's
// hands, a knock and a crack), and a filtered-noise riser that runs while
// they grow and cuts as the last one stops; each bar's stop is a bell
// pitched by its place (A5, F5, D5, A4; over the answer's E5, A5 E5 C5
// A4), heard where its bar ends, and hk's rings brighter, with the crew's
// hands. The groove's hits rest around each stop, so none flams with a
// bell or passes for the stop. The bracket ticks in and a glint runs
// across hk's bar. Between two races the bars draw back on a reversed
// breath, with a spring when a tool changes rank; with one, a softer glint
// crosses hk's bar in the middle of the long hold, and a soft ping runs
// down the bars as its hold's caption lands. With no race to run (no
// facts, or none backed), hk's own check run plays instead, and each of
// its ✔ rows lands on a pluck.

import type { Part } from ".";
import type { ReelFacts } from "../facts";
import { checkAll } from "../kit/screens";
import { type RaceRun, raceRuns } from "../race-timing";
import { F0_EACH, F0_FIRST } from "../scenes/race";
import { EXIT_WIPE, HOLD_GLINT, REORDER, RESET } from "../scenes/race-chart";
import type { Section } from "../timeline";
import { WHIP_AT, WHIP_END } from "../whip";
import { ANSWER, bassBars, C, CHOP, CHORD, chopBars, chordBars, DM, drumBars, G, type Hush, LEAD, M, melody, STOMP_FULL } from "./grooves";
import { ad, hz, line, type Mix, type Pt, sweep } from "./mix";
import { bassBar, boing, ding, fiddlePluck, fiddleSlide, flick, gangClap, knock, OOMPAH, panX, ping, riser, shimmer, stomp, tick, whoosh } from "./sounds";

/** The bars grow from x 700, 1000 px for the chart's full scale. */
const barEnd = (median: number, axis: number): number => 700 + (median / axis) * 1000;

/**
 * A stop's bell by finishing place: A5, F5, D5, A4, and on down D dorian's
 * Dm for more tools. The second race stops under the answer's held E5, so
 * its bells ring the notes that sit with it: A5, E5, C5, A4.
 */
const PLACE = [81, 77, 74, 69, 65, 62];
const PLACE_OVER_E = [81, 76, 72, 69, 64, 62];

/** The stubs crouch for a sixteenth before each start (scenes/race-chart.ts CROUCH). */
const CROUCH = 0.25;
/**
 * A glint runs across hk's bar a sixteenth after the bracket, over 0.36 s;
 * with one race, another crosses it at HOLD_GLINT over 0.4 s, and one runs
 * down every bar from b9.5, a 32nd apart (scenes/race-chart.ts drawGlints).
 */
const GLINT_AFTER = 0.25;
const GLINT_LEN = 0.36;
const HOLD_GLINT_LEN = 0.4;
const ROLL_CALL = { from: 9.5, each: 0.125 } as const;

/** F0's ✔ step rows climb D dorian's Dm7 in the order they land. */
const F0_NOTES = [62, 65, 69, 72, 74, 77, 81];
/** Plucks that land on the same frame strum this far apart. */
const STRUM = 0.004;

/** How near a bar's stop the groove's hits rest, and how near a rival's a hit counts as landing with its bell. */
const CLEAR = 0.06;
const WITH = 0.008;

/**
 * Where the groove's boots, hands, tambourine and chops rest: within CLEAR
 * of a bar's stop, where a hit would flam with the bell or pass for the stop
 * itself, early or late. A hit that lands with a rival's bell still strikes
 * with it; around hk's stop every hit rests, for its own clap.
 */
function clearings(s: Section, facts: ReelFacts | null): Hush {
  const stops = raceRuns(facts).flatMap((run) => Object.entries(run.stops).map(([key, b]) => ({ hk: key === "hk", t: s.beat(b) })));
  return (t) => {
    const near = stops.filter((stop) => Math.abs(t - stop.t) < CLEAR);
    return near.some((stop) => stop.hk) || (near.length > 0 && !near.some((stop) => Math.abs(t - stop.t) <= WITH));
  };
}

function race(m: Mix, s: Section, run: RaceRun, places: readonly number[]): void {
  const start = s.beat(run.start);
  const last = Math.max(...Object.values(run.stops));
  // The start: the stubs crouch as the fiddle's bow digs in, swelling into
  // the start as it slides up to the tune's A; then the gun as the bars
  // leave the axis, a boot, the crew's hands, a wooden knock on D4 and a
  // whip's crack; and a riser under them.
  m.duck(start, 0.3, 0.12);
  stomp(m, start, 0.9);
  gangClap(m, start, 0.8, "sfx");
  knock(m, start, hz(62), 0.35, 0.1, 0.14);
  flick(m, start, 0.15, 0.2);
  const glide = Math.min(0.45, (last - run.start) * 0.35);
  const crouch = s.beat(run.start - CROUCH);
  fiddleSlide(m, crouch, s.beat(run.start + glide), hz(62), hz(69), 0.06, 0.22, 0.15, start - crouch);
  riser(m, start, s.beat(last) - 0.004, { vel: 0.55, fifths: false, roll: false });

  // Each stop: a bell by place, where its bar ends. hk's rings brighter,
  // with the crew's hands, as the gun's are; the groove rests around it
  // (clearings), so nothing else strikes there.
  run.order.forEach((key, place) => {
    const row = run.race.rows.find((r) => r.key === key);
    if (!row) return;
    const t = s.beat(run.stops[key]);
    const pan = panX(barEnd(row.median, run.race.axis));
    const note = places[Math.min(place, places.length - 1)];
    m.duck(t, 0.12, 0.1);
    ding(m, t, hz(note), key === "hk" ? 0.1 : 0.07, pan, 1.1, 0.35);
    if (key === "hk") gangClap(m, t, 0.8, "sfx");
  });

  // The bracket draws from hk's bar end to the fastest rival's.
  const hk = run.race.rows[0];
  const rival = run.race.claim.rival;
  const hkEnd = barEnd(hk.median, run.race.axis);
  tick(m, s.beat(last), 2600, 0.35, panX((hkEnd + barEnd(rival.median, run.race.axis)) / 2), 0.15);
  // A glint runs across hk's bar.
  shimmer(m, s.beat(last + GLINT_AFTER), s.beat(last + GLINT_AFTER) + GLINT_LEN, 0.022, panX(hkEnd));
}

/**
 * One race only: a softer glint crosses hk's bar in the middle of the long
 * hold, and as the hold's caption lands a glint runs down every bar, a soft
 * ping each.
 */
function rollCall(m: Mix, s: Section, run: RaceRun): void {
  const hk = run.race.rows.find((r) => r.key === "hk");
  if (hk) shimmer(m, s.at(HOLD_GLINT), s.at(HOLD_GLINT) + HOLD_GLINT_LEN, 0.016, panX(barEnd(hk.median, run.race.axis)));
  run.race.rows.forEach((row, i) => {
    const t = s.beat(ROLL_CALL.from + i * ROLL_CALL.each);
    ping(m, t, hz(PLACE[Math.min(i, PLACE.length - 1)] + 12), row.key === "hk" ? 0.035 : 0.02, 0.4, { pan: panX(barEnd(row.median, run.race.axis)), send: 0.4 });
  });
}

/** Between the races: the bars draw back on a reversed breath, and a spring if a tool changes rank. */
function reset(m: Mix, s: Section, before: RaceRun, after: RaceRun): void {
  const t0 = s.at(RESET[0]);
  const t1 = s.at(REORDER[1]);
  // It is gone 20 ms before the second race starts, so the start lands clean.
  const cut = t1 - 0.02;
  const env: Pt[] = [[t0 - 0.05, 0], [cut - 0.03, 0.08, "lin"], [cut, 0.0001, "exp"], [cut + 0.004, 0]];
  whoosh(m, env, sweep(t0 - 0.05, 900, cut, 3600), 1.2, { pan: line(t0, 0.4, cut, -0.2), send: 0.15 });
  const rank = (r: RaceRun) => r.race.rows.map((x) => x.key).join();
  // The spring is short, so it has rung out as the second race starts.
  if (rank(before) !== rank(after)) boing(m, s.at(REORDER[0]), 0.1, hz(57), 0.2);
}

/**
 * No race to run: hk's own check run, with a pluck for each ✔ row on the
 * frame it lands, climbing the Dm7 across the run; rows that land together
 * strum.
 */
function checkRun(m: Mix, s: Section): void {
  let step = 0;
  checkAll.forEach((rows, f) => {
    if (!f) return;
    const before = new Set<string>(checkAll[f - 1]);
    const t = s.at(F0_FIRST + F0_EACH * f);
    let strum = 0;
    for (const r of rows.slice(1)) {
      if (before.has(r) || !r.startsWith("✔ ")) continue;
      if (r.startsWith("✔ files")) {
        tick(m, t, 1900, 0.14, -0.3, 0.08);
        continue;
      }
      const note = F0_NOTES[Math.min(step, F0_NOTES.length - 1)];
      step++;
      fiddlePluck(m, t + STRUM * strum++, hz(note), 0.06, -0.3 + 0.08 * step, 0.25, { bus: "sfx" });
    }
  });
  // 7/7: the run is done, on the frame the counter reads it, just after that frame's last row.
  const done = checkAll.findIndex((rows) => rows[0].endsWith("7/7"));
  if (done > 0) ping(m, s.at(F0_FIRST + F0_EACH * done) + STRUM, hz(86), 0.04, 0.7, { pan: 0.1, send: 0.35 });
}

export const part: Part = {
  // The peak.
  level: 1.1,
  cues(m, s, facts: ReelFacts | null) {
    // b0 to b1: the whip clears, and the chart rides in from the right on the air.
    whoosh(m, [[WHIP_AT - 0.03, 0], [WHIP_AT + 0.03, 0.09], [WHIP_END, 0.0001, "exp"], [WHIP_END + 0.004, 0]], sweep(WHIP_AT, 4200, WHIP_END, 900), 1.2, { pan: line(WHIP_AT, 0.65, WHIP_END, 0.05), send: 0.2 });

    const runs = raceRuns(facts);
    if (runs.length) {
      runs.forEach((run, i) => race(m, s, run, i ? PLACE_OVER_E : PLACE));
      if (runs.length > 1) reset(m, s, runs[0], runs[1]);
      else rollCall(m, s, runs[0]);
    } else {
      checkRun(m, s);
    }

    // The labels, figures and details wipe from left to right (or, with no
    // race, the pane fades), and the bars ease into the morph's capsules.
    const [wipe0, wipe1] = EXIT_WIPE.map((t) => s.at(t));
    flick(m, wipe0, -0.4, 0.25);
    whoosh(m, ad(wipe0, wipe0 + 0.15, 0.1, wipe1 + 0.03), sweep(wipe0, 1400, wipe1, 4200), 1.4, { pan: line(wipe0, -0.6, wipe1, 0.6), send: 0.2 }, "white");
  },
  drums: (m, s, facts) => drumBars(m, s, [STOMP_FULL, STOMP_FULL, STOMP_FULL, [[0, 8, 10, 12], [4], [2, 6]]], 0.82, [], clearings(s, facts)),
  bass(m, s) {
    bassBars(m, s, [DM, C, DM], OOMPAH, 3);
    bassBar(m, s.bar(3), ...G, [
      [0, 3, 0, 1],
      [4, 3, 7, 0.8],
      [8, 3, 0, 1],
      [12, 2, 7, 0.8],
    ]);
  },
  lead(m, s) {
    // The chorus, sung out louder than anywhere else in the reel.
    melody(m, s.start, M, 1.15 * LEAD);
    melody(m, s.beat(8), ANSWER, 1.15 * LEAD);
  },
  pads(m, s, facts) {
    // Bar 2 is a Csus4: the tune's F4 falls on its downbeat.
    const chords = [CHORD.Dm, CHORD.Csus4, CHORD.Dm, CHORD.G];
    chordBars(m, s, chords);
    // Everyone plays at the peak: the fiddle chops the off-beats too, until the boots take the eighths.
    chopBars(m, s, chords, 1.5 * CHOP, 0, 3, undefined, clearings(s, facts));
  },
};
