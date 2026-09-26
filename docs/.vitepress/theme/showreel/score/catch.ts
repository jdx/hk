// Catch: a later commit is caught on the hook. The groove drops to half
// time over a heartbeat and a D2 pedal, under a dark Dm; the ✗ on b3 cuts
// the drums and the chord. The pedal holds through the snag and the yank,
// and the stomp returns on b6 with the bass bouncing again.

import type { Part } from ".";
import { BED, CHORD, DM, HALF, heartbeat, STOMP, stompBar } from "./grooves";
import { bassBar, bassRun, concertina, pad } from "./sounds";

export const part: Part = {
  drums(m, s) {
    stompBar(m, s.bar(0), HALF);
    heartbeat(m, s, 0, 3);
    stompBar(m, s.bar(1), [[8], [12]]);
    stompBar(m, s.bar(2), STOMP);
  },
  bass(m, s) {
    bassRun(m, [[s.start, s.beat(6), 38, 1]], 0.04, 260, 0.3);
    bassBar(m, s.bar(1), ...DM, [
      [8, 3, 0, 1],
      [12, 3, 7, 0.8],
    ]);
    bassBar(m, s.bar(2), ...DM);
  },
  pads(m, s) {
    pad(m, s.start, s.beat(3) - 0.05, [50, 57, 62, 65], 0.05, 900, 0.3, 0.05, 150);
    const o = { attack: 0.05, sustain: 0.8, release: 0.08, bright: 2000, pan: -0.3, send: 0.22 };
    concertina(m, s.beat(6), s.bar(2), CHORD.Dm, BED, o);
    concertina(m, s.bar(2), s.end, CHORD.Dm, BED, o);
  },
};
