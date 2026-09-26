// Everywhere: the same steps in a commit, at a terminal, and in CI. The
// full groove over Dm | C | Dm, with the tambourine's hats on the eighths
// under the pulses (b3 to b5). At b10.5 the drums drop for the riser into
// the whip, all but a pickup boot on b11.5, and the bass holds A, the
// dominant, into the race's Dm.

import type { Part } from ".";
import { bassBars, C, CHORD, chordBars, DM, drumBars, STOMP_FULL } from "./grooves";
import { bassBar, bassRun, hat, OOMPAH } from "./sounds";

export const part: Part = {
  drums(m, s) {
    drumBars(m, s, [STOMP_FULL, STOMP_FULL, [[0, 8, 14], [4], [2, 6], [7]]]);
    for (let b = 3; b < 5; b += 0.5) hat(m, s.beat(b), 0.8);
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
