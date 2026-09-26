// Lanes: the seven steps run as bars on four file lanes. The full groove,
// Dm | C | Dm | G, with the tambourine. Bar 2 breaks at b6.5 into muted
// ghost jingles over a bass pedal on A, the clamp on b8 brings the stomp
// back, and over the G the concertina resolves motif M on D, which lifts
// into restore.

import type { Part } from ".";
import { C, CHORD, chordBars, DM, type Drums, drumBars, G, HOME, melody, STOMP_FULL } from "./grooves";
import { bassBar, bassRun } from "./sounds";

/** Bar 2: the stomp until b6.5, then only the tambourine, muted, on every sixteenth. */
const BREAK: Drums = [[0, 8], [4], [2, 6], [10, 11, 12, 13, 14, 15]];

export const part: Part = {
  drums: (m, s) => drumBars(m, s, [STOMP_FULL, BREAK, STOMP_FULL]),
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
  pads: (m, s) => chordBars(m, s, [CHORD.Dm, CHORD.C, CHORD.Dm, CHORD.G]),
};
