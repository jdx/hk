// Config: builtin names stream past and seven steps land in hk.pkl. The
// groove enters on the downbeat: boots and the crew's claps, the oom-pah
// bass on Dm | C | Dm, the concertina's chords, and over bars 2 and 3 its
// tune, motif M, which comes home to D on the Dm.
//
// The picture's cues, timed from the scene's beat map (scenes/config.ts):
// the tambourine shivers under the rivers of names; prettier and zizmor
// pop out of them left and right, and zizmor plops back in; the card draws
// on with a paper flick; the fiddle climbs D dorian a sixteenth at a time
// as the seven steps land in the card; the closing brace stamps in on a
// double stop that closes the climb; a warm
// pluck marks the amended builtin; and the card folds down into the hk.pkl
// chip with a flick and a soft click as it seats.

import type { Part } from ".";
import { lerp } from "../math";
import { T_BAND, T_CARD, T_FOLD, T_LAND, T_LOGO_OUT, T_POP, T_SINK, T_STAMP, T_TAKEOFF } from "../scenes/config";
import { bassBars, C, CHORD, chordBars, DM, drumBars, M, melody, STOMP } from "./grooves";
import { ad, hz, line, sweep } from "./mix";
import { bloop, fiddlePluck, flick, jingleRoll, knock, panX, ping, pop, puff, stamp, thump, tick, whoosh } from "./sounds";

/** The card spans x 560 to 1800; the chip sits at x 960. */
const CARD = panX(1180);

/** The seven steps climb D dorian, D4 up to C5, one a sixteenth as they land (T_LAND). */
const CLIMB = [62, 64, 65, 67, 69, 71, 72];

/** zizmor is back among its river's names 80% of the way through its sink (scenes/config.ts T_SINK): the plop. */
const PLOP = lerp(T_SINK[0], T_SINK[1], 0.8);

export const part: Part = {
  // The groove has only just come in: it sits back.
  level: 0.8,
  cues(m, s) {
    // The wordmark recedes as the groove comes in: a breath of air.
    whoosh(m, ad(s.start, s.beat(0.3), 0.06, s.at(T_LOGO_OUT)), sweep(s.start, 2400, s.at(T_LOGO_OUT), 700), 1.2, { send: 0.25 });
    // The tambourine shivering under the rivers of names, until they start
    // to ebb (b6), and gone before the closing brace stamps in.
    jingleRoll(m, s.beat(0.5), s.beat(6.25), 0.3);

    // prettier (x 420) and zizmor (x 1500) pop out of the rivers, each
    // with a ring pulse: a cork pop and a small bright ping.
    const pops = s.at(T_POP);
    pop(m, pops, hz(74), 0.2, panX(420), 0.22);
    pop(m, pops + 0.012, hz(81), 0.18, panX(1500), 0.22);
    ping(m, pops + 0.02, hz(86), 0.025, 0.35, { pan: panX(420), send: 0.4 });
    ping(m, pops + 0.03, hz(93), 0.022, 0.35, { pan: panX(1500), send: 0.4 });
    // zizmor sinks back into its river with a small plop.
    bloop(m, s.at(PLOP), hz(69), 0.05, panX(1500), 0.18);

    // The card's outline draws on: a paper flick, with the stroke's air after it.
    const [card0, card1] = T_CARD.map((t) => s.at(t));
    flick(m, card0, CARD, 0.3);
    whoosh(m, ad(card0, card0 + 0.1, 0.07, card1 + 0.05), sweep(card0, 3000, card1 + 0.05, 1600), 2, { pan: line(card0, panX(560), card1, panX(1800)), send: 0.2 }, "white");

    // The seven steps lift out of the rivers and fly half a beat into the
    // card, where each lands on a fiddle pluck, climbing D dorian.
    CLIMB.forEach((n, i) => {
      const t = s.at(T_LAND[i]);
      flick(m, s.at(T_TAKEOFF[i]), panX(340), 0.035);
      fiddlePluck(m, t, hz(n), 0.085 + 0.005 * i, line(t, panX(340), t + 0.25, CARD), 0.22, { bus: "sfx", bright: 10 });
    });

    // The closing brace stamps in and closes the climb: a knock on C5 and a
    // double stop, G4 and E5, with the tune's C5 and the C chord, which
    // carry the stamp over the tune on a small speaker.
    const st = s.at(T_STAMP);
    m.duck(st, 0.4, 0.1);
    stamp(m, st, 1.2, panX(640));
    thump(m, st, 0.35, 150, 60, 0.25, { pan: panX(640) });
    knock(m, st, hz(72), 0.3, panX(640), 0.12);
    fiddlePluck(m, st + 0.003, hz(67), 0.1, panX(640), 0.25, { bus: "sfx", len: 0.45, bright: 9 });
    fiddlePluck(m, st + 0.007, hz(76), 0.085, panX(700), 0.25, { bus: "sfx", len: 0.45, bright: 9 });

    // The warm band sweeps ruff-format's lines: a builtin, amended. A warm
    // double stop on the fiddle, under a breath of air down the card.
    const band = s.at(T_BAND);
    fiddlePluck(m, band, hz(69), 0.1, CARD, 0.3, { bus: "sfx", len: 0.7, bright: 10 });
    fiddlePluck(m, band + 0.006, hz(74), 0.085, CARD + 0.1, 0.3, { bus: "sfx", len: 0.7, bright: 10 });
    whoosh(m, ad(band, band + 0.15, 0.06, band + 0.5), sweep(band, 900, band + 0.5, 1800), 1.4, { pan: CARD, send: 0.3 });

    // The card folds down into the chip, and seats.
    const [fold0, fold1] = T_FOLD.map((t) => s.at(t));
    flick(m, fold0, CARD, 0.3);
    whoosh(m, ad(fold0, fold0 + 0.35, 0.09, fold1 + 0.03), sweep(fold0, 2200, fold1, 600), 1.3, { pan: line(fold0, CARD, fold1, 0), send: 0.2 });
    tick(m, fold1, 1900, 0.45, 0, 0.12);
    puff(m, fold1, 0.07, 0, false);
  },
  drums: (m, s) => drumBars(m, s, [STOMP]),
  bass: (m, s) => bassBars(m, s, [DM, C, DM]),
  // The hook's first statement, stated softly: the race sings it out.
  lead: (m, s) => melody(m, s.beat(4), M, 0.08),
  pads: (m, s) => chordBars(m, s, [CHORD.Dm, CHORD.C, CHORD.Dm]),
};
