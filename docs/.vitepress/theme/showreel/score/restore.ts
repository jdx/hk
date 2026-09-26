// Restore: the fixes are staged and the edit comes back, then the commit
// lands. Half time in bar 1, the stomp again from bar 2, over Am | G | Dm.

import type { Part } from ".";
import { AM, bassBars, CHORD, chordBars, DM, drumBars, G, HALF, STOMP } from "./grooves";

export const part: Part = {
  drums: (m, s) => drumBars(m, s, [HALF, STOMP]),
  bass: (m, s) => bassBars(m, s, [AM, G, DM]),
  pads: (m, s) => chordBars(m, s, [CHORD.Am, CHORD.G, CHORD.Dm]),
};
