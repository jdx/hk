// Commit: `git commit` slams in and hk's hook starts. The stomp carries on
// over Dm | C | Dm, and the fiddle chops the and of 2 and the and of 4.

import type { Part } from ".";
import { bassBars, C, CHORD, chopBars, chordBars, DM, drumBars, STOMP } from "./grooves";

const CHORDS = [CHORD.Dm, CHORD.C, CHORD.Dm];

export const part: Part = {
  drums: (m, s) => drumBars(m, s, [STOMP]),
  bass: (m, s) => bassBars(m, s, [DM, C, DM]),
  pads(m, s) {
    chordBars(m, s, CHORDS);
    chopBars(m, s, CHORDS);
  },
};
