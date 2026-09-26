// Stash: the unstaged line is sliced off into a stash. The stomp carries on
// over Dm | Am | Dm, and the concertina answers motif M over bars 2 and 3.

import type { Part } from ".";
import { AM, ANSWER, bassBars, CHORD, chordBars, DM, drumBars, melody, STOMP } from "./grooves";

export const part: Part = {
  drums: (m, s) => drumBars(m, s, [STOMP]),
  bass: (m, s) => bassBars(m, s, [DM, AM, DM]),
  lead: (m, s) => melody(m, s.beat(4), ANSWER),
  pads: (m, s) => chordBars(m, s, [CHORD.Dm, CHORD.Am, CHORD.Dm]),
};
