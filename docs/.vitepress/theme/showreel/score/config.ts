// Config: builtin names stream past and seven steps land in hk.pkl. The
// groove enters on the downbeat: boots and the crew's claps, the oom-pah
// bass on Dm | C | Dm, the concertina's chords, and over bars 2 and 3 its
// tune, motif M, which comes home to D on the Dm.

import type { Part } from ".";
import { bassBars, C, CHORD, chordBars, DM, drumBars, M, melody, STOMP } from "./grooves";
import { jingleRoll } from "./sounds";

export const part: Part = {
  cues(m, s) {
    // The tambourine shivering under the rivers of names.
    jingleRoll(m, s.beat(0.5), s.beat(2.5), 0.6);
  },
  drums: (m, s) => drumBars(m, s, [STOMP]),
  bass: (m, s) => bassBars(m, s, [DM, C, DM]),
  lead: (m, s) => melody(m, s.beat(4), M),
  pads: (m, s) => chordBars(m, s, [CHORD.Dm, CHORD.C, CHORD.Dm]),
};
