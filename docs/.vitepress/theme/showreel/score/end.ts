// The end card, the resolve: after a sixteenth of silence the downbeat
// lands with a boot and the crew's hands, a long low D, and a Dm(add9) on
// the concertina, pressed hard and let go slowly. An afterglow pad enters
// with the tagline, a closing heave-ho (boot, boot, hands) comes on b8 to
// b9, and then only the afterglow remains, gone before the last frame.

import type { Part } from ".";
import { hz, perc } from "./mix";
import { concertina, gangClap, pad, stomp } from "./sounds";

/** Dm(add9): D3 A3 D4 E4 F4 A4. */
const RESOLVE = [50, 57, 62, 64, 65, 69];

export const part: Part = {
  drums(m, s) {
    stomp(m, s.start, 1);
    gangClap(m, s.start, 1);
    stomp(m, s.beat(8), 0.9);
    stomp(m, s.beat(8.5), 0.8);
    gangClap(m, s.beat(9), 1);
  },
  bass(m, s) {
    // The last oom: a low D that rings out over two bars, with the grit that carries it to small speakers.
    const t = s.start;
    const v = m.voice(perc(t, 0.075, 0.008, 3.2), { bus: "music", hold: true });
    if (!v) return;
    const sat = v.shaper(m.sh.sat, v.filter("lowpass", 300, -3));
    v.osc("sine", hz(38), 0.7, sat);
    v.osc("triangle", hz(50), 0.12, sat);
    v.osc("sawtooth", hz(38), 0.3, v.filter("bandpass", 210, 1.1, v.filter("lowpass", 420, -3)));
  },
  pads(m, s) {
    concertina(m, s.start, s.beat(4), RESOLVE, 0.075, { attack: 0.015, sustain: 0.35, release: 1.2, bright: 5000, send: 0.3 });
    pad(m, s.beat(2), s.end - 0.55, [...RESOLVE], 0.05, 1300, 0.6, 0.35);
  },
};
