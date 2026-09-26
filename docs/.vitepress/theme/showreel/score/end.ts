// The end card, the resolve: after a sixteenth of silence the downbeat
// lands with a boot and the crew's hands, a long low D, and a Dm(add9) on
// the concertina, pressed hard and let go slowly. An afterglow pad enters
// with the tagline, a closing heave-ho (boot, boot, hands) comes on b8 to
// b9, and then only the afterglow remains, gone before the last frame.
//
// The picture's cues: the barb clicks home on the downbeat with the reel's
// third ship's bell; the glint runs down the leg on tiny bells and the
// sparkle at the point pings; "hk" rises on a pluck of D5; the tagline
// lands on a breath; `mise use hk` types a key a 32nd, the spaces lower;
// the address arrives on a bell a D higher; and the logo's small reaction
// swing creaks once.

import type { Part } from ".";
import { BOX_IN, KEY_EVERY, T_CLICK, T_HK, T_KEY0, T_PROMPT, T_REACT, T_SPARKLE, T_TAGLINE, T_URL } from "../scenes/end";
import { BEAT } from "../timeline";
import { ad, hz, line, perc, sweep } from "./mix";
import { bells, concertina, creak, ding, fiddlePluck, flick, gangClap, pad, panX, ping, shimmer, stomp, tick, whoosh } from "./sounds";

/** Dm(add9): D3 A3 D4 E4 F4 A4. */
const RESOLVE = [50, 57, 62, 64, 65, 69];

/** The logo sits right of centre (LOGO_END, x 1196 to 1644); the point's sparkle at (1522, 432). */
const LOGO = panX(1420);
const POINT = panX(1522);
/** The type is on the left, from x 160. */
const TYPE = panX(360);
/** Typed a key a 32nd after the prompt. */
const COMMAND = "mise use hk";

export const part: Part = {
  cues(m, s) {
    // The resolve: the barb clicks home, and the ship's bell rings twice.
    const click = s.at(T_CLICK);
    m.duck(click, 0.5, 0.35);
    tick(m, click, 2400, 0.35, POINT, 0.12);
    bells(m, click, hz(74), hz(81), 0.075, LOGO);
    // The glint runs down the leg on tiny bells, and the sparkle peaks at the point.
    shimmer(m, click + 0.03, s.at(T_SPARKLE), 0.035, POINT);
    ping(m, s.at(T_SPARKLE), hz(93), 0.08, 0.6, { pan: POINT, send: 0.45 });

    // "hk" rises on a pluck of D5.
    const hk = s.at(T_HK);
    whoosh(m, ad(hk - 0.1, hk + 0.05, 0.03, hk + 0.2), sweep(hk - 0.1, 700, hk + 0.15, 2200), 1.3, { pan: TYPE, send: 0.3 });
    fiddlePluck(m, hk, hz(74), 0.12, TYPE, 0.35, { bus: "sfx", len: 0.8, bright: 9 });
    // The tagline rises word by word, a 32nd apart, and lands.
    const tag = s.at(T_TAGLINE);
    whoosh(m, ad(tag - 0.375, tag - 0.1, 0.02, tag + 0.45), sweep(tag - 0.375, 1200, tag + 0.1, 2600), 1.2, { pan: line(tag - 0.375, -0.6, tag, 0), send: 0.35 });

    // The install box draws in.
    flick(m, s.at(BOX_IN[0]), panX(460), 0.25);
    // `$`, then `mise use hk` a key a 32nd; the space bar is lower.
    tick(m, s.at(T_PROMPT), 2100, 0.12, panX(200), 0.08);
    [...COMMAND].forEach((ch, i) => {
      const t = s.at(T_KEY0 + i * KEY_EVERY);
      if (ch === " ") tick(m, t, 1100, 0.12, panX(260 + 38 * i), 0.08);
      else tick(m, t, 2300 + 260 * ((i * 5) % 3), 0.1, panX(260 + 38 * i), 0.08);
    });

    // hk.jdx.dev, on a bell a D above the pluck.
    ding(m, s.at(T_URL), hz(86), 0.05, TYPE, 1.4, 0.4);

    // The logo's small reaction swing (swingDeg from T_REACT, 3°, τ a beat),
    // one soft creak at its first end, where the damped sine first peaks.
    const react = s.at(T_REACT) + (Math.atan(2 * Math.PI) / (2 * Math.PI)) * BEAT;
    creak(m, react - 0.05, react + 0.14, 0.4, LOGO);
  },
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
