// Open: the hook drops in on its line while a pen draws hk. No groove yet:
// the sea washes in under a concertina drone on D and A through bar 1, a
// fiddle plucks each stroke as the pen draws it (D4 F4 A4 C5, climbing the
// Dm7 the reel lives in), and the reel's ratchet clicks as the line pays
// out under the falling hook. The dock is a wooden knock, the leg creaks
// as it cocks, the barb's click is the reel's first hit (the ship's bell,
// struck twice), and bar 2 brings half-time boots under the logo's swing,
// the timber creaking at each end of it.
//
// Every cue is timed from the scene's own beat map (scenes/open.ts).

import type { Part } from ".";
import { inOutSine, inQuad, keys, outQuad, smoothstep } from "../math";
import { STROKE_AT, T_CLICK, T_DOCK, T_LET_GO, T_SPARKLE } from "../scenes/open";
import { BAR, BEAT, beat, type Section } from "../timeline";
import { ad, hz, sweep } from "./mix";
import { bells, concertina, creak, fiddlePluck, knock, panX, pen, ping, ratchetAlong, stomp, tick, wave, whoosh } from "./sounds";

/** The drone's reeds: D3 and A3, an open fifth that is neither major nor minor yet. */
const DRONE = [50, 57];

/** Logo x (LOGO_OPEN: 2.5 px a unit about x 960) to a stereo position. */
const at = (ux: number): number => panX(960 + (ux - 80) * 2.5);

/** Where each stroke sits (logo units), and the fiddle note the first four pluck. */
const STROKES = [
  { x: 28, note: 62 }, // hStem
  { x: 44, note: 65 }, // hShoulder
  { x: 92, note: 69 }, // kStem
  { x: 109, note: 72 }, // kArm
  { x: 118 }, // kLeg, the dock
] as const;

/** The hook hangs at x 1090. */
const HOOK = panX(1090);

/**
 * The hook's drop, px (scenes/open.ts dropAt): it picks up speed, pays out
 * fast and slows past its dock, rebounds on the line's stretch, and seats.
 */
const drop = keys([
  [0, -300],
  [beat(0.2), -273, inQuad],
  [beat(2.3), 16, outQuad],
  [beat(2.7), -5, inOutSine],
  [T_DOCK, 0, inQuad],
]);

/**
 * The swing's apexes from the click: swingDeg(t, b4, 8°, 1 bar) eased out
 * over b5.5 to b7. A damped sine peaks a little before its quarter period,
 * at atan(ωτ)/ω, then every half period. Returns [time, |angle| in degrees].
 */
function apexes(s: Section): [number, number][] {
  const w = (2 * Math.PI) / BEAT;
  const first = Math.atan(w * BAR) / w;
  const out: [number, number][] = [];
  for (let k = 0; ; k++) {
    const d = first + (k * BEAT) / 2;
    const t = s.at(T_CLICK) + d;
    if (t >= s.beat(7)) break;
    out.push([t, 8 * Math.exp(-d / BAR) * (1 - smoothstep(s.beat(5.5), s.beat(7), t))]);
  }
  return out;
}

export const part: Part = {
  cues(m, s) {
    // The sea under bar 1, draining away under the barb's click.
    wave(m, s.start, s.beat(5.5), 0.07, -0.15);

    // The pen draws each stroke, and a fiddle plucks as each of the first four starts.
    STROKES.forEach((k, i) => {
      const [from, to] = STROKE_AT[i];
      pen(m, s.at(from), s.at(to), i < 4 ? 0.035 : 0.028, at(k.x));
      if ("note" in k) fiddlePluck(m, s.at(from), hz(k.note), 0.07, at(k.x), 0.3, { bus: "sfx", len: 0.9 });
    });

    // The reel's ratchet clicks as the line pays out, crowding while the
    // hook falls fast and spreading as it slows toward the dock.
    ratchetAlong(m, (t) => drop(t - s.start), s.start, s.at(T_DOCK), 7, 0.045, 3600, 2200, HOOK);
    // The dock: a wooden knock on D4.
    knock(m, s.at(T_DOCK), hz(62), 0.32, HOOK, 0.18);
    // The line goes slack and whips up out of frame.
    whoosh(m, ad(s.at(T_LET_GO), s.at(T_LET_GO) + 0.17, 0.07, s.at(T_CLICK) + 0.03), sweep(s.at(T_LET_GO), 700, s.at(T_CLICK), 4800), 2.2, { pan: HOOK, send: 0.2 });
    // The docked leg cocks back against the swing before the click kicks it.
    creak(m, s.beat(3.64), s.beat(3.96), 0.14, at(118));

    // The barb clicks in: the reel's first hit, the ship's bell twice (D5
    // then A5), and a high ping as the sparkle peaks at the point.
    tick(m, s.at(T_CLICK), 2400, 0.4, at(106), 0.1);
    bells(m, s.at(T_CLICK), hz(74), hz(81), 0.075, at(100));
    ping(m, s.at(T_SPARKLE), hz(93), 0.05, 0.5, { pan: at(106), send: 0.45 });

    // The timber creaks at each end of the swing, softer as it settles and
    // leaning to the side the k's leg swings to.
    apexes(s).forEach(([t, deg], i) => {
      if (deg < 0.8) return;
      creak(m, t - 0.05, t + 0.13, 0.4 * (deg / 8), at(118) + (i % 2 ? -0.12 : 0.12));
    });
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
