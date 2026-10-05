// The sea at the foot of the frame, under the title, the choruses and the
// outro: four swells drawn as hairlines in hk's cyan, farther ones fainter,
// rolling left and heaving on the song's beat, a crest every other beat.
// Below the captions' baselines, so the words stay on a quiet ground.

import { PALETTE } from "../bible";
import { rgba } from "../color";
import { clamp, TAU } from "../math";
import { beatAt } from "./song";

/** Each swell, back to front: its rest line, height, wavelength and speed. */
const SWELLS = [
  { y: 968, amp: 5, len: 520, speed: 26, phase: 0.3, alpha: 0.16 },
  { y: 996, amp: 7, len: 440, speed: 38, phase: 2.1, alpha: 0.22 },
  { y: 1028, amp: 9, len: 380, speed: 52, phase: 4.4, alpha: 0.3 },
  { y: 1062, amp: 12, len: 330, speed: 70, phase: 1.2, alpha: 0.38 },
] as const;

export interface SeaOptions {
  /** 0 below the frame, 1 at rest: the tide comes in. */
  rise?: number;
  alpha?: number;
}

/** The sea at song time `t`. */
export function drawSea(ctx: CanvasRenderingContext2D, t: number, o: SeaOptions = {}): void {
  const a = clamp(o.alpha ?? 1);
  const rise = clamp(o.rise ?? 1);
  if (a <= 0 || rise <= 0) return;
  // One heave per two beats, highest on the beat the boots land on.
  const heave = 0.5 + 0.5 * Math.cos((TAU * beatAt(t)) / 2);
  ctx.save();
  ctx.lineWidth = 2;
  ctx.lineJoin = "round";
  SWELLS.forEach((s, i) => {
    const k = TAU / s.len;
    const drop = (1 - rise) * (140 + 30 * i);
    const lift = heave * (3 + 2 * i);
    ctx.strokeStyle = rgba(PALETTE.cyan, a * s.alpha);
    ctx.beginPath();
    for (let x = -10; x <= 1930; x += 10) {
      const u = k * (x + s.speed * t) + s.phase;
      const y = s.y + drop - lift + s.amp * Math.sin(u) + 0.35 * s.amp * Math.sin(2.3 * u + 1.7);
      if (x === -10) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  });
  ctx.restore();
}
