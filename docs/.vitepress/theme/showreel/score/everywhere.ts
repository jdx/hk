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
// arpeggio that gives way to the next slam. Then the
// contrast the scene is about: three checks read one file together (C5 E5
// G5 plucked at once, on the left), and three fixes take turns on it (the
// same notes one at a time, on the right, the write lock shutting for each
// and springing open between them). A soft bell when both columns are
// done, and the whip: a riser and a breath of air that winds up, then
// tears away left into the race.

import type { Part } from ".";
import { ALL_DONE, CHECK_DONE, CHECK_GO, COLUMN_IN, FIX_HOLDS, FOLD, FOLD_LEN, IMPACT, LAUNCH, PULSE_FLY, PULSES, TICKS } from "../scenes/everywhere-timing";
import { WHIP_AT, WHIP_OUT, WHIP_START } from "../whip";
import { bassBars, C, CHORD, chordBars, DM, type Drums, drumBars, STOMP_FULL } from "./grooves";
import { ad, hz, line, type Pt, sweep } from "./mix";
import { bassBar, bassRun, blip, ding, fiddlePluck, hat, jingle, knock, OOMPAH, padlock, panX, ping, riser, stab, stomp, whoosh } from "./sounds";

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

/** The check column's pills start at x 540; the fix column's run x 1400 to 1760. */
const CHECK = panX(700);
const FIX = panX(1580);
/** The padlocks: read at x 470, write at x 1330. */
const READ = panX(470);
const WRITE = panX(1330);
/**
 * The rows, prettier, trailing-whitespace and newlines: checks share them,
 * fixes take turns. C5 E5 G5: the C chord's own notes, where the checks
 * start, and the Dm's seventh, ninth and eleventh under the last two fixes.
 */
const ROWS = [72, 76, 79];

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

    // Three checks read README.md at once: one chord, and the read lock.
    const go = s.beat(CHECK_GO);
    padlock(m, go, true, 0.25, READ);
    ROWS.forEach((n, i) => fiddlePluck(m, go + 0.004 * i, hz(n), 0.06, CHECK, 0.25, { bus: "sfx", len: 0.6 }));
    // Each check's ✔ lands as its read ends, newlines first.
    CHECK_DONE.forEach((b, i) => ping(m, s.beat(b), hz(ROWS[i] + 12), 0.035, 0.4, { pan: CHECK, send: 0.3 }));
    padlock(m, s.beat(Math.max(...CHECK_DONE)) + 0.01, false, 0.16, READ);

    // Three fixes take turns on it: the write lock shuts for each and
    // springs open as it lets go, and each fix starts on its note.
    FIX_HOLDS.forEach(([from, to], i) => {
      padlock(m, s.beat(from), true, 0.25, WRITE);
      fiddlePluck(m, s.beat(from) + 0.006, hz(ROWS[i]), 0.11, FIX, 0.25, { bus: "sfx", len: 0.6, bright: 10 });
      padlock(m, s.beat(to), false, 0.2, WRITE);
      if (to < ALL_DONE) ping(m, s.beat(to) + 0.01, hz(ROWS[i] + 12), 0.03, 0.35, { pan: FIX, send: 0.3 });
    });

    // Both columns ✔: a soft bell, left and right.
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
    // The first panel slams a sixteenth after the downbeat on its own boot,
    // so the downbeat's boot is only a light step; the third slam's boot is
    // the one on 3, so the groove leaves it out rather than doubling it.
    stomp(m, s.start, 0.45);
    const [, hands, jingles, ghosts] = STOMP_FULL;
    const landing: Drums = [[10], hands, jingles, ghosts];
    drumBars(m, s, [landing, STOMP_FULL, [[0, 8, 14], [4], [2, 6], [7]]]);
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
