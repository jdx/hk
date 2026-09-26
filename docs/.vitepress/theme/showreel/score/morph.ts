// Morph, the breakdown: the bars set into the logo. The drums drop to a
// heartbeat on b0 and b2 and the bass drops out; the concertina swells from
// Dm to Am under a riser that began in the race's last bar, and everything,
// the room included (audio.ts), stops dead for the last sixteenth before
// the end card's downbeat.

import type { Part } from ".";
import { sec } from "../timeline";
import { heartbeat } from "./grooves";
import { X } from "./mix";
import { concertina, riser } from "./sounds";

/** The breath before the resolve: nothing sounds from here to the end card's downbeat. */
export const GAP = sec("morph").end - X;

export const part: Part = {
  cues(m, s) {
    // From the race's b14; the roll doubles to sixteenths on b2 and to 32nds on b3.
    riser(m, sec("race").beat(14), GAP, { doubles: [s.beat(2), s.beat(3)], vel: 0.6 });
  },
  drums: (m, s) => heartbeat(m, s),
  pads(m, s) {
    const o = { sustain: 1, bright: 3000, send: 0.3 };
    concertina(m, s.start, s.beat(2), [50, 57, 62, 65], 0.045, { ...o, attack: 0.9, release: 0.06 });
    concertina(m, s.beat(2), GAP - 0.024, [45, 57, 60, 64], 0.06, { ...o, attack: 0.8, release: 0.02 });
  },
};
