// Restore: the fixes are staged and the edit comes back, then the commit
// lands. Half time in bar 1, the stomp again from bar 2, over Am | G | Dm.
//
// The picture's cues: the bars draw back into their labels; four `✓ staged`
// stamps hit the labels in 32nds, each lighter, each with a knock down the
// Am; the split opens with a
// divider drawing down the middle and the fixed lines sliding across it;
// the tray creaks open and the TODO line zips into the worktree copy,
// landing with a bloop; the cards close up and hk's last rows tick in; the
// main line draws on, the new commit shoots along it on a rising breath and
// lands on b7 with the reel's second bell, on the G. The beats are the
// scene's own (scenes/restore.ts).

import type { Part } from ".";
import { T_ABSORB, T_BLOOP, T_CHARGE, T_CLOSE, T_COMMITTED, T_HEAD, T_LAND, T_LID, T_LIFT, T_RESTORED, T_RETRACT0, T_SPLIT, T_STAMPS, T_ZIP0, T_ZIP1 } from "../scenes/restore";
import { AM, bassBars, CHORD, chordBars, DM, drumBars, G, HALF, STOMP } from "./grooves";
import { ad, hz, line, sweep } from "./mix";
import { bells, bloop, creak, gangClap, knock, panX, puff, stamp, stomp, tick, whoosh, zip } from "./sounds";

/** The lane labels sit at x 160 to 560; the tray at x 1520 to 1760. */
const LABELS = panX(360);
const TRAY = panX(1640);
/** The worktree card is x 1040 to 1760. */
const WORKTREE = panX(1400);

/**
 * Beats of the scene (scenes/restore.ts) it keeps to itself: the fixed
 * lines copy across from b2.6875, the tray's lid shuts over b3.5 to b3.75,
 * main draws on over b5.5 to b6.5.
 */
const COPY = 2.6875;
const LID_SHUT = 3.75;
const LINE = [5.5, 6.5] as const;

export const part: Part = {
  cues(m, s) {
    // The finished bars swell for a moment, then a warm edge sweeps them
    // back into their labels, right to left. Its air is gone a 32nd before
    // the stamps, which carry the labels' moment.
    const charge = s.at(T_CHARGE);
    const back = s.at(T_RETRACT0);
    const absorb = s.at(T_ABSORB);
    whoosh(m, ad(charge, back, 0.02, back + 0.08), sweep(charge, 600, back, 1400), 1.3, { send: 0.2, hold: false });
    whoosh(m, ad(back, absorb - 0.1, 0.07, absorb), sweep(back, 2600, absorb, 700), 1.3, { pan: line(back, 0.45, absorb, LABELS), send: 0.2 });

    // `✓ staged` stamps each lane label in a 32nd cascade, lighter after
    // the first, each with a small wooden knock stepping down the Am, A4
    // to E4, that a small speaker hears.
    m.duck(s.at(T_STAMPS[0]), 0.3, 0.12);
    [1, 0.6, 0.5, 0.42].forEach((v, i) => {
      stamp(m, s.at(T_STAMPS[i]), 0.85 * v, LABELS);
      knock(m, s.at(T_STAMPS[i]), hz([69, 67, 64, 64][i]), 0.3 * v, LABELS + 0.05 * i, 0.12);
    });
    // README.md dims to `= HEAD`: its stamp goes quiet.
    tick(m, s.at(T_HEAD), 700, 0.3, LABELS, 0.1);

    // The divider draws down the middle, and the fixed lines slide across it.
    const split = s.at(T_SPLIT);
    whoosh(m, ad(split, split + 0.1, 0.045, split + 0.5), sweep(split, 5000, split + 0.5, 2400), 2.5, { send: 0.25 }, "white");
    whoosh(m, ad(s.beat(COPY), s.beat(COPY + 0.2), 0.06, s.at(T_LID) + 0.05), sweep(s.beat(COPY), 900, s.at(T_LID), 2000), 1.4, { pan: line(s.beat(COPY), panX(520), s.at(T_LID), WORKTREE), send: 0.2 });

    // The tray creaks open and the TODO line rises out of it, zips into the
    // worktree card as its line 11 and settles with a bloop; the lid knocks shut.
    const lid = s.at(T_LID);
    creak(m, lid - 0.04, s.at(T_ZIP0), 0.35, TRAY);
    whoosh(m, ad(lid, s.at(T_ZIP0) - 0.02, 0.04, s.at(T_ZIP0) + 0.02), sweep(lid, 700, s.at(T_ZIP0), 1800), 1.4, { pan: TRAY, send: 0.2, hold: false });
    zip(m, s.at(T_ZIP0), s.at(T_ZIP1) - 0.015, 10, 0.12, line(s.at(T_ZIP0), TRAY, s.at(T_ZIP1), WORKTREE));
    bloop(m, s.at(T_BLOOP), hz(74), 0.12, WORKTREE);
    knock(m, s.beat(LID_SHUT), hz(55), 0.12, TRAY, 0.1);

    // The split closes onto the staged side, and the strip drops in from above.
    const close = s.at(T_CLOSE);
    whoosh(m, ad(close, close + 0.3, 0.07, s.at(T_RESTORED)), sweep(close, 700, s.at(T_RESTORED), 1900), 1.1, { send: 0.25 });
    puff(m, s.at(T_RESTORED) - 0.01, 0.06, 0, false);
    // hk's last row, then git's commit: soft terminal ticks.
    tick(m, s.at(T_RESTORED), 3000, 0.3, -0.2, 0.08);
    tick(m, s.at(T_COMMITTED), 2600, 0.25, -0.1, 0.08);

    // main draws on from left to right.
    whoosh(m, ad(s.beat(LINE[0]), s.beat(LINE[1] - 0.3), 0.04, s.beat(LINE[1] + 0.1)), sweep(s.beat(LINE[0]), 1500, s.beat(LINE[1]), 3000), 2.2, { pan: line(s.beat(LINE[0]), -0.6, s.beat(LINE[1]), 0.6), send: 0.2 }, "white");
    // The commit lifts off as a dot and shoots along the line...
    const lift = s.at(T_LIFT);
    const land = s.at(T_LAND);
    whoosh(m, ad(lift, land - 0.025, 0.1, land + 0.03), sweep(lift, 500, land, 4200), 1.6, { pan: line(lift, -0.5, land, panX(1400)), send: 0.25, hold: false });
    // ...and lands at x 1400 on the G: the reel's second bell.
    bells(m, land, hz(74), hz(81), 0.09, panX(1400));
    stomp(m, land, 0.9, panX(1400) * 0.5);
    gangClap(m, land, 1);
  },
  drums: (m, s) => drumBars(m, s, [HALF, STOMP]),
  bass: (m, s) => bassBars(m, s, [AM, G, DM]),
  pads: (m, s) => chordBars(m, s, [CHORD.Am, CHORD.G, CHORD.Dm]),
};
