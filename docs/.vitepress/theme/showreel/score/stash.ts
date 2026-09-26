// Stash: the unstaged line is sliced off into a stash. The stomp carries on
// over Dm | Am | Dm, and the concertina answers motif M over bars 2 and 3.
//
// The picture's cues: the main.py card rises on a breath and lands with a
// soft pat; a blade slices off the TODO line with a flick, a bright shing
// and a spray of sparks; the strip tears away and falls into the tray, whose
// lid knocks shut on the boot at b3 as hk's row updates on a pluck; a soft
// rising breath follows the read band down the card; each squiggle under a
// formatting problem wobbles a pluck (F5 G5 A5 C6); then the strip leaves, the
// card shrinks into its lane label, the other labels type on, and the
// tracks draw across on an arpeggio, D4 F4 A4 D5. The blade's and the
// strip's timing are the scene's own (scenes/stash-peel.ts), and so are
// the rest of its beats (scenes/stash-timing.ts).

import type { Part } from ".";
import { bladeX, CUT, FLY, INTO, PEEL, SPARKS_AT } from "../scenes/stash-peel";
import { CARD_OUT, LID, LOCKS as LOCKS_IN, READ, ROW_AT, SQUIGGLES, STRIP_OUT, TRACKS, TYPE } from "../scenes/stash-timing";
import { AM, ANSWER, bassBars, CHORD, chordBars, DM, drumBars, melody, STOMP } from "./grooves";
import { ad, hz, line, type Pt, sweep } from "./mix";
import { crackle, fiddlePluck, flick, knock, padlock, panX, puff, shing, stomp, thump, tick, whoosh } from "./sounds";

/** The card spans x 660 to 1260, and the tray x 1520 to 1760. */
const CARD = panX(960);
const TRAY = panX(1640);
/** The lane labels sit at x 160 onwards; the padlocks at x 606. */
const LABELS = panX(300);
const LOCKS = panX(606);

/** The labels type a key a 32nd: eight keys cover the three names' visible typing. */
const TYPE_KEYS = 8;

export const part: Part = {
  level: 0.92,
  cues(m, s) {
    // b0 to b1.5: the pane folds up into the strip, and the card rises on a
    // spring and lands at b1.5.
    whoosh(m, ad(s.start, s.beat(0.6), 0.05, s.beat(1.4)), sweep(s.start, 1800, s.beat(1.4), 500), 1, { send: 0.2 });
    whoosh(m, ad(s.beat(0.5), s.beat(1.3), 0.09, s.beat(1.6)), sweep(s.beat(0.5), 500, s.beat(1.5), 2200), 1.2, { pan: CARD, send: 0.25 });
    puff(m, s.beat(1.5), 0.09, CARD, false);
    thump(m, s.beat(1.5), 0.3, 170, 80, 0.12, { pan: CARD });

    // The slice: the blade runs along the TODO line's top edge in a
    // sixteenth, a flick and a bright shing that travel with it, and throws
    // sparks off the card's right edge.
    const cut = s.beat(CUT.from);
    m.duck(cut, 0.2, 0.12);
    flick(m, cut, panX(CUT.x0), 0.12);
    shing(m, cut + 0.01, 0.12, panX(1060));
    const along: Pt[] = [0, 0.25, 0.5, 0.75, 1].map((u) => {
      const b = CUT.from + u * (CUT.to - CUT.from);
      return [s.beat(b), panX(bladeX(b))];
    });
    whoosh(m, ad(cut, cut + 0.03, 0.05, s.beat(CUT.to) + 0.04), sweep(cut, 5000, s.beat(CUT.to), 8000), 3, { pan: along, send: 0.2, hold: false }, "white");
    const sparks = s.beat(SPARKS_AT);
    for (let i = 0; i < 5; i++) {
      tick(m, sparks + 0.028 * i + 0.01 * Math.sin(i * 7.1), 5200 + 700 * ((i * 3) % 5), 0.12 / (1 + 0.4 * i), panX(1270 + 40 * i), 0.2);
    }

    // The strip peels up behind the blade with a tear, the tray's lid swings
    // up on its hinge, and the strip flies in an arc into the slot.
    crackle(m, s.beat(PEEL.from), s.beat(PEEL.to) - s.beat(PEEL.from) + 0.03, 0.13, panX(1100));
    tick(m, s.beat(LID.open) + 0.03, 900, 0.12, TRAY, 0.1);
    whoosh(m, ad(s.beat(FLY.from), s.beat(FLY.to) - 0.1, 0.07, s.beat(INTO.to)), sweep(s.beat(FLY.from), 2400, s.beat(INTO.to), 700), 1.5, { pan: line(s.beat(FLY.from), panX(1100), s.beat(INTO.from), TRAY), send: 0.2 });

    // b3: the lid knocks shut on a boot, and hk's row updates: ✔ stash.
    m.duck(s.beat(LID.slam), 0.2, 0.12);
    knock(m, s.beat(LID.slam), hz(55), 0.34, TRAY, 0.12);
    stomp(m, s.beat(LID.slam), 0.7, TRAY * 0.5);
    fiddlePluck(m, s.beat(ROW_AT) + 0.01, hz(74), 0.11, -0.2, 0.25, { bus: "sfx", bright: 9 });

    // The read band sweeps down the staged card: what the linters see.
    whoosh(m, ad(s.beat(READ.from), s.beat(READ.to - 0.5), 0.06, s.beat(READ.to + 0.1)), sweep(s.beat(READ.from), 500, s.beat(READ.to), 2600), 1.6, { pan: CARD, send: 0.35 });

    // A warm squiggle under each formatting problem: a pluck that wobbles,
    // F5 G5 A5 C6, over the answer's D5 and C5 rather than against them. The
    // last lands on Am's C, an octave over the tune's.
    [77, 79, 81, 84].forEach((n, i) => {
      fiddlePluck(m, s.beat(SQUIGGLES[i].at), hz(n), 0.12, panX(820), 0.25, { bus: "sfx", wobble: 0.03, len: 0.4, bright: 10 });
    });

    // Into the lanes. The strip slides up out of frame; the card shrinks
    // into lane 3's label; the other labels type on; the tracks draw across
    // on an arpeggio, D4 F4 A4 D5; the padlocks come up open.
    whoosh(m, ad(s.beat(STRIP_OUT.from), s.beat(STRIP_OUT.to - 0.4), 0.08, s.beat(STRIP_OUT.to)), sweep(s.beat(STRIP_OUT.from), 900, s.beat(STRIP_OUT.to), 3600), 1.3, { send: 0.2 });
    whoosh(m, ad(s.beat(CARD_OUT.from), s.beat(CARD_OUT.to - 0.3), 0.08, s.beat(CARD_OUT.to)), sweep(s.beat(CARD_OUT.from), 2600, s.beat(CARD_OUT.to), 800), 1.3, { pan: line(s.beat(CARD_OUT.from), CARD, s.beat(CARD_OUT.to), LABELS), send: 0.2 });
    for (let i = 0; i < TYPE_KEYS; i++) tick(m, s.beat(TYPE.from + i * TYPE.each), 2500 + 250 * (i % 3), 0.25, LABELS + 0.05 * (i % 2), 0.06);
    [62, 65, 69, 74].forEach((n, i) => {
      fiddlePluck(m, s.beat(TRACKS.from + i * TRACKS.each), hz(n), 0.07, panX(700 + 330 * i), 0.25, { bus: "sfx", bright: 9 });
    });
    for (let lane = 0; lane < 4; lane++) padlock(m, s.beat(LOCKS_IN.from + lane * LOCKS_IN.each), false, 0.06, LOCKS);
  },
  drums: (m, s) => drumBars(m, s, [STOMP]),
  bass: (m, s) => bassBars(m, s, [DM, AM, DM]),
  // The answer to config's soft statement, still held back for the race.
  lead: (m, s) => melody(m, s.beat(4), ANSWER, 0.09),
  pads: (m, s) => chordBars(m, s, [CHORD.Dm, CHORD.Am, CHORD.Dm]),
};
