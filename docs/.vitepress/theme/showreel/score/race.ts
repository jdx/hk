// Race: the published benchmark. The peak: the full stomp and the
// tambourine over Dm | C | Dm | G, and the concertina's whole chorus,
// motif M and then its answer, which ends on D over the G and lifts into
// the morph. Bar 4 turns to boots on the eighths at b14, and everything
// drops at b15.5 under the riser (score/morph.ts). The race's cues will take
// the facts, as its bars do (Part's third argument).

import type { Part } from ".";
import { ANSWER, bassBars, C, CHORD, chordBars, DM, drumBars, G, M, melody, STOMP_FULL } from "./grooves";
import { bassBar, OOMPAH } from "./sounds";

export const part: Part = {
  drums: (m, s) => drumBars(m, s, [STOMP_FULL, STOMP_FULL, STOMP_FULL, [[0, 8, 10, 12], [4], [2, 6]]]),
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
    melody(m, s.start, M);
    melody(m, s.beat(8), ANSWER);
  },
  pads: (m, s) => chordBars(m, s, [CHORD.Dm, CHORD.C, CHORD.Dm, CHORD.G]),
};
