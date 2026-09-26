// Open: the hook drops in on its line while a pen draws hk. No groove yet:
// the sea washes in under a concertina drone on D and A through bar 1, and
// bar 2 brings half-time boots under the logo's swing.

import type { Part } from ".";
import { concertina, stomp, wave } from "./sounds";

/** The drone's reeds: D3 and A3, an open fifth that is neither major nor minor yet. */
const DRONE = [50, 57];

export const part: Part = {
  cues(m, s) {
    // The sea under bar 1, draining away under the barb's click.
    wave(m, s.start, s.beat(5.5), 0.09, -0.15);
  },
  drums(m, s) {
    // Half-time boots on b4 and b6.
    stomp(m, s.beat(4), 0.85);
    stomp(m, s.beat(6), 0.7);
  },
  pads(m, s) {
    // The bellows open slowly from frame 0, press again on the downbeat of
    // bar 2, and let go into config's first chord.
    const o = { bright: 2200, pan: -0.1, send: 0.3 };
    concertina(m, s.start, s.beat(4), DRONE, 0.05, { ...o, attack: 1.6, sustain: 1, release: 0.05 });
    concertina(m, s.beat(4), s.end - 0.02, DRONE, 0.05, { ...o, attack: 0.08, sustain: 0.75, release: 0.1 });
  },
};
