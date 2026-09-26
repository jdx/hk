// Morph, the breakdown: the bars set into the logo. The drums drop to a
// heartbeat on b0 and b2 and the bass drops out; the concertina swells from
// Dm to Am under a riser that began in the race's last bar, and everything,
// the room included (audio.ts), stops dead for the last sixteenth before
// the end card's downbeat.
//
// The picture's cues, timed from the scene's own (scenes/morph-cues.ts):
// a light turns each bar cyan on a soft rising ping, each bar settling
// into its stroke is a soft pluck stepping back down, and the pen from the
// open draws the rest of the mark.

import type { Part } from ".";
import { FILL, LAND, PEN } from "../scenes/morph-cues";
import { sec } from "../timeline";
import { heartbeat } from "./grooves";
import { hz, X } from "./mix";
import { concertina, fiddlePluck, panX, pen, ping, riser } from "./sounds";

/** The breath before the resolve: nothing sounds from here to the end card's downbeat. */
export const GAP = sec("morph").end - X;

/** As a light turns each bar cyan it rings a little higher: A5 C6 D6 E6, soft. */
const FILLS = [81, 84, 86, 88];
/**
 * Each bar landing on its stroke (the h's stem, the k's stem, the k's arm,
 * the leg's straight run) is a soft pluck stepping down D dorian, E5 D5 C5
 * A4, to the Am's root, heard where its stroke sits.
 */
const LANDINGS = [
  { note: 76, x: 1216 },
  { note: 74, x: 1467 },
  { note: 72, x: 1534 },
  { note: 69, x: 1542 },
] as const;

export const part: Part = {
  cues(m, s) {
    // From the race's b14; the roll doubles to sixteenths on b2 and to 32nds on b3.
    riser(m, sec("race").beat(14), GAP, { doubles: [s.beat(2), s.beat(3)], vel: 0.34 });
    FILL.forEach((t, i) => ping(m, s.at(t), hz(FILLS[i]), 0.02, 0.3, { pan: panX(1060), send: 0.45 }));
    LANDINGS.forEach((a, i) => fiddlePluck(m, s.at(LAND[i]), hz(a.note), 0.05, panX(a.x), 0.4, { bus: "sfx", len: 0.45 }));
    // The pen from the open draws on the h's shoulder and the rest of the k's leg.
    pen(m, s.at(PEN[0]), s.at(PEN[1]), 0.03, panX(1420));
  },
  drums: (m, s) => heartbeat(m, s),
  pads(m, s) {
    const o = { sustain: 1, bright: 3000, send: 0.3 };
    concertina(m, s.start, s.beat(2), [50, 57, 62, 65], 0.045, { ...o, attack: 0.9, release: 0.06 });
    concertina(m, s.beat(2), GAP - 0.024, [45, 57, 60, 64], 0.06, { ...o, attack: 0.8, release: 0.02 });
  },
};
