// Everywhere: the same steps in a commit, at a terminal, and in CI. The
// full groove over Dm | C | Dm, with the tambourine's hats on the eighths
// under the pulses (b3 to b5). At b10.5 the drums drop for the riser into
// the whip, all but a pickup boot on b11.5, and the bass holds A, the
// dominant, into the race's Dm.
//
// The picture's cues, timed from the scene's own beat map
// (scenes/everywhere-timing.ts): three panels slam down a beat apart,
// left, middle and right, each on a boot, a concertina stab (Dm, C, Dm),
// a knock and the tambourine, and its eight ✔ rows tick on in a 32nd
// arpeggio that gives way to the next slam. Then one step in two modes:
// under hk check ruff-format reads the file (the read lock, a pluck on A4)
// and fails on a dull low knock, and its diff unfolds a tick a row, low for
// a removal and high for an addition. Under hk fix the same command reads
// the file, which ruff has fixed first, on the same A4; its diff comes up
// on a quick breath of air as the read lock lets go, and a soft stamp
// lands as the write lock shuts; the removals are struck in one scratch
// and the additions settle together on a rising run, a pluck each, and the
// ✔ rings as the lock springs open. A soft bell when both columns are
// done, and the whip: a riser and a breath of air that winds up, then
// tears away left into the race.

import type { Part } from ".";
import { RUFF_FORMAT_DIFF, RUFF_FORMAT_FIX_DIFF } from "../kit/card";
import { ALL_DONE, APPLY, CHECK_DONE, CHECK_GO, COLUMN_IN, DIFF_IN, DIFF_ROW, FIX_DIFF_ROW, FIX_DONE, FIX_GO, FIX_PATCH, FIX_WRITE, FOLD, FOLD_LEN, IMPACT, LAUNCH, PULSE_FLY, PULSES, TICKS } from "../scenes/everywhere-timing";
import { WHIP_AT, WHIP_OUT, WHIP_START } from "../whip";
import { bassBars, C, CHORD, chordBars, DM, drumBars, STOMP_FULL } from "./grooves";
import { ad, hz, line, type Pt, sweep } from "./mix";
import { bassBar, bassRun, blip, ding, fiddlePluck, hat, jingle, knock, OOMPAH, padlock, panX, ping, riser, stab, stamp, stomp, thump, tick, whoosh } from "./sounds";

/**
 * The three panels, x centres: commit, terminal, CI. Each slam's stab and
 * knock, and its eight ✔ rows' arpeggio. The last slam lands while the
 * middle panel's rows are still ticking, so it hits harder.
 */
const PANELS = [
  { x: 420, chord: CHORD.Dm, arp: [62, 65, 69, 72, 74, 77, 81, 86], stab: 0.075, knock: 0.22 },
  { x: 960, chord: CHORD.C, arp: [60, 64, 67, 72, 76, 79, 84, 88], stab: 0.075, knock: 0.22 },
  { x: 1500, chord: CHORD.Dm, arp: [65, 69, 72, 74, 77, 81, 84, 86], stab: 0.12, knock: 0.3 },
] as const;

/** The check column's pill and diff sit about x 600; the fix column's about x 1460. */
const CHECK = panX(600);
const FIX = panX(1460);
/** The padlocks: hk check's at x 470, hk fix's at x 1330. */
const CHECK_LOCK = panX(470);
const FIX_LOCK = panX(1330);
/** The additions settle on the Dm's own notes, rising, a pluck for each of the fix card's: D5 F5 A5 D6. */
const SETTLE = [74, 77, 81, 86].slice(0, RUFF_FORMAT_FIX_DIFF.filter((l) => l.startsWith("+")).length);

export const part: Part = {
  cues(m, s) {
    // The panels slam down out of the chip, which kicks as each leaves.
    PANELS.forEach((p, i) => {
      const t = s.beat(IMPACT[i]);
      const pan = panX(p.x);
      whoosh(m, ad(s.beat(LAUNCH[i]), t - 0.005, 0.04, t + 0.02), sweep(s.beat(LAUNCH[i]), 2400, t, 900), 1.5, { pan: line(s.beat(LAUNCH[i]), 0, t, pan), send: 0.12, hold: false });
      m.duck(t, 0.22, 0.14);
      stomp(m, t, 0.85, pan * 0.5);
      stab(m, t, p.chord.map((n) => n + 12), p.stab, pan);
      knock(m, t, hz(p.chord[0] + 12), p.knock, pan, 0.14);
      jingle(m, t, 0.6, 0.15, pan);
      // Its eight ✔ rows tick on, a 32nd apart; the rows that would land
      // on the next slam or a 32nd before it give way to it, so it lands
      // out of a breath.
      TICKS[i].forEach((b, k) => {
        if (IMPACT.some((at) => at - b > -1e-6 && at - b < 0.125 + 1e-6)) return;
        fiddlePluck(m, s.beat(b), hz(p.arp[k]), 0.045, pan, 0.2, { bus: "sfx", len: 0.3, bright: 9 });
      });
    });

    // Pulses run down the connectors on the eighths (the hats), each
    // reaching the panels with a soft blip.
    for (const b of PULSES) blip(m, s.beat(b + PULSE_FLY), hz(93), 0.03);

    // The panels roll up into the chip, left to right, and two columns draw in.
    FOLD.forEach((b, i) => {
      whoosh(m, ad(s.beat(b), s.beat(b + FOLD_LEN * 0.7), 0.06, s.beat(b + FOLD_LEN) + 0.03), sweep(s.beat(b), 2600, s.beat(b + FOLD_LEN), 900), 1.2, { pan: line(s.beat(b), panX(PANELS[i].x), s.beat(b + FOLD_LEN), 0), send: 0.2 });
    });
    COLUMN_IN.forEach((b, i) => whoosh(m, ad(s.beat(b), s.beat(b + 0.25), 0.04, s.beat(b + 0.45)), sweep(s.beat(b), 800, s.beat(b + 0.45), 2400), 1.4, { pan: i ? FIX : CHECK, send: 0.2 }));

    // hk check: ruff-format reads src/main.py under the read lock, on A4…
    const go = s.beat(CHECK_GO);
    padlock(m, go, true, 0.25, CHECK_LOCK);
    fiddlePluck(m, go + 0.004, hz(69), 0.08, CHECK, 0.25, { bus: "sfx", len: 0.5 });
    // …and fails: a dull low knock and a soft thump, and the lock springs open.
    const fail = s.beat(CHECK_DONE);
    knock(m, fail, hz(45), 0.32, CHECK, 0.1);
    thump(m, fail, 0.22, 95, 55, 0.18, { pan: CHECK });
    padlock(m, fail + 0.01, false, 0.16, CHECK_LOCK);
    // Its diff unfolds a row every 1/16 beat: a tick a row, low for a removal, high for an addition.
    RUFF_FORMAT_DIFF.forEach((line, r) => {
      const f = line.startsWith("-") ? 1400 : line.startsWith("+") ? 3200 : 2200;
      tick(m, s.beat(DIFF_IN + r * DIFF_ROW) + 0.004, f, 0.12, CHECK, 0.05);
    });

    // hk fix: the same command reads the file under a read lock, on the same A4…
    const read = s.beat(FIX_GO);
    padlock(m, read, true, 0.2, FIX_LOCK);
    fiddlePluck(m, read + 0.004, hz(69), 0.07, FIX, 0.25, { bus: "sfx", len: 0.5 });
    // …its diff comes up on a quick breath of air as the read lock lets go…
    const patch = s.beat(FIX_PATCH);
    const whole = s.beat(FIX_PATCH + RUFF_FORMAT_FIX_DIFF.length * FIX_DIFF_ROW);
    padlock(m, patch + 0.01, false, 0.14, FIX_LOCK);
    whoosh(m, ad(patch, patch + 0.03, 0.03, whole + 0.02), sweep(patch, 1800, whole, 4200), 2, { pan: FIX, send: 0.15, hold: false }, "white");
    // …and a soft stamp lands as ruff-format takes the write lock, on D5.
    const write = s.beat(FIX_WRITE);
    stamp(m, write, 0.4, FIX);
    padlock(m, write, true, 0.25, FIX_LOCK);
    fiddlePluck(m, write + 0.006, hz(74), 0.1, FIX, 0.25, { bus: "sfx", len: 0.5, bright: 10 });
    // hk applies it: the removals struck in one scratch, then the additions
    // settle together on a rising run, a pluck each.
    const strike = s.beat(APPLY[0]);
    whoosh(m, ad(strike, strike + 0.02, 0.035, strike + 0.14), sweep(strike, 6000, strike + 0.14, 3500), 3, { pan: FIX, send: 0.1, hold: false }, "white");
    SETTLE.forEach((n, i) => fiddlePluck(m, s.beat(APPLY[0] + 0.3 + i / 16), hz(n), 0.05, FIX, 0.25, { bus: "sfx", len: 0.4 }));
    // ✔, and the lock springs open.
    ping(m, s.beat(FIX_DONE) + 0.01, hz(86), 0.04, 0.4, { pan: FIX, send: 0.3 });
    padlock(m, s.beat(FIX_DONE), false, 0.2, FIX_LOCK);

    // Both runs are over, check's ✗ and fix's ✔: a soft bell, left and right.
    ding(m, s.beat(ALL_DONE), hz(81), 0.035, CHECK, 1.2, 0.4);
    ding(m, s.beat(ALL_DONE) + 0.012, hz(86), 0.025, FIX, 1.1, 0.4);

    // The whip. A riser, and air that winds up with the columns easing
    // right, then tears away left as they whip out.
    riser(m, WHIP_START, WHIP_AT, { vel: 0.4, doubles: [s.beat(11), s.beat(11.5)] });
    // The streaks peak on the bar line and race off left as the race's air comes in from the right.
    const out = WHIP_AT - WHIP_OUT;
    const env: Pt[] = [[WHIP_START, 0], [out, 0.015], [WHIP_AT, 0.12, "exp"], [WHIP_AT + 0.28, 0.0001, "exp"], [WHIP_AT + 0.284, 0]];
    whoosh(m, env, [[WHIP_START, 700], [WHIP_AT, 5200, "exp"], [WHIP_AT + 0.28, 2000, "exp"]], 1.1, { pan: [[out, 0.15], [WHIP_AT, -0.55], [WHIP_AT + 0.28, -0.8]], send: 0.15 });
  },
  drums(m, s) {
    // Each panel slams on its own boot, so the groove's boots rest wherever
    // a slam lands (the third's is the one on 3). The first lands a 32nd
    // after the downbeat, so the downbeat's boot gives way to a light step.
    const late = IMPACT[0] > 0 && IMPACT[0] < 0.25;
    if (late) stomp(m, s.start, 0.45);
    drumBars(m, s, [STOMP_FULL, STOMP_FULL, [[0, 8, 14], [4], [2, 6], [7]]], 0.82, late ? [0, ...IMPACT] : IMPACT);
    // Soft: on 3.5 and 4.5 they fall with the tambourine.
    for (let b = 3; b < 5; b += 0.5) hat(m, s.beat(b), 0.5);
  },
  bass(m, s) {
    bassBars(m, s, [DM, C], OOMPAH, 2);
    bassBar(m, s.bar(2), ...DM, [
      [0, 3, 0, 1],
      [4, 3, 7, 0.8],
      [8, 2, 0, 0.9],
    ]);
    bassRun(m, [[s.beat(10.5), s.end - 0.03, 33, 0.8]], 0.04, 300, 0.75);
  },
  pads: (m, s) => chordBars(m, s, [CHORD.Dm, CHORD.C, CHORD.Dm]),
};
