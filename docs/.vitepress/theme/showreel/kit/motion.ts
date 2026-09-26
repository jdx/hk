// Motion every scene shares, on the caller's clock (seconds): pops, typing,
// the terminal cursor's blink, thrown arcs and the sparks that travel them,
// and settles that land exactly on their mark, so a scene rests on its
// handoff frame to the pixel. Every function is a pure function of its
// arguments.

import { PALETTE } from "../bible";
import { mix, rgba } from "../color";
import { glow } from "../fx";
import { lerp, progress, spring, TAU } from "../math";
import { BEAT } from "../timeline";

export interface Pt {
  x: number;
  y: number;
}
export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** A rect tweened toward another. */
export const lerpRect = (a: Rect, b: Rect, k: number): Rect => ({
  x: lerp(a.x, b.x, k),
  y: lerp(a.y, b.y, k),
  w: lerp(a.w, b.w, k),
  h: lerp(a.h, b.h, k),
});

/** An actor popping in at `at`: its scale, a spring from 0 that overshoots a little. */
export const popIn = (t: number, at: number, freq = 4.5, damping = 0.5): number => spring(t - at, freq, damping);

/** How many characters of `text` are typed at `t`, typing from `at` for `dur`. */
export const typedChars = (text: string, t: number, at: number, dur: number): number =>
  Math.floor(text.length * progress(at, at + dur, t) + 1e-9);

/**
 * A terminal's block cursor, blinking with the beat: on for a beat, off for
 * the next. Pass GLOBAL `t`, so a cursor that crosses a bar line keeps its
 * phase: on while floor(t / BEAT) is even.
 */
export const cursorOn = (t: number): boolean => Math.floor(t / BEAT + 1e-9) % 2 === 0;

// Thrown arcs.

/** A quadratic Bézier: a thrown arc or a gentle bend. */
export interface Curve {
  a: Pt;
  c: Pt;
  b: Pt;
}

/** An arc from a to b bulging `lift` of its length to the left of travel (up for a rightward throw). */
export function arc(a: Pt, b: Pt, lift = 0.25): Curve {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const flip = dx < 0 ? -1 : 1;
  return { a, b, c: { x: (a.x + b.x) / 2 + dy * lift * flip, y: (a.y + b.y) / 2 - Math.abs(dx) * lift } };
}

export function curveAt(k: Curve, u: number): Pt {
  const v = 1 - u;
  return {
    x: v * v * k.a.x + 2 * v * u * k.c.x + u * u * k.b.x,
    y: v * v * k.a.y + 2 * v * u * k.c.y + u * u * k.b.y,
  };
}

/** The direction of travel at u, not normalized. */
export function curveTangent(k: Curve, u: number): Pt {
  return {
    x: 2 * (1 - u) * (k.c.x - k.a.x) + 2 * u * (k.b.x - k.c.x),
    y: 2 * (1 - u) * (k.c.y - k.a.y) + 2 * u * (k.b.y - k.c.y),
  };
}

export interface SparkOptions {
  color?: string;
  /** Head radius px. */
  size?: number;
  /** Tail length along the curve, in u. */
  trail?: number;
  alpha?: number;
}

/** A spark travelling a curve, its head at u with a fading tail: a key handed on, a pulse. */
export function drawSpark(ctx: CanvasRenderingContext2D, k: Curve, u: number, o: SparkOptions = {}): void {
  const a = o.alpha ?? 1;
  if (a <= 0 || u < 0 || u > 1) return;
  const color = o.color ?? PALETTE.cyanBright;
  const size = o.size ?? 7;
  const trail = o.trail ?? 0.18;
  const head = curveAt(k, u);
  ctx.save();
  ctx.globalAlpha *= a;
  glow(ctx, head.x, head.y, size * 6, color, 0.7);
  const n = 10;
  ctx.lineCap = "round";
  for (let i = n; i >= 1; i--) {
    const p0 = curveAt(k, Math.max(0, u - (trail * i) / n));
    const p1 = curveAt(k, Math.max(0, u - (trail * (i - 1)) / n));
    ctx.strokeStyle = rgba(color, 0.9 * (1 - i / (n + 1)));
    ctx.lineWidth = size * 1.6 * (1 - i / (n + 1));
    ctx.beginPath();
    ctx.moveTo(p0.x, p0.y);
    ctx.lineTo(p1.x, p1.y);
    ctx.stroke();
  }
  ctx.fillStyle = mix(color, "#ffffff", 0.6);
  ctx.beginPath();
  ctx.arc(head.x, head.y, size, 0, TAU);
  ctx.fill();
  ctx.restore();
}

// Settles that are home, to the last bit, when they say they are.

/**
 * 0 before `at`, then up to 1 over `dur` seconds, overshooting by about
 * `over` of the way (a back-out curve) and landing exactly on 1: a pop or a
 * snap that is home when it says it is.
 */
export function land(t: number, at: number, dur: number, over = 0.12): number {
  const p = progress(at, at + dur, t);
  if (p <= 0 || p >= 1) return p;
  // outBack with the overshoot solved for: s 1.70158 gives 10%.
  const s = 1.70158 * (over / 0.1);
  return 1 + (s + 1) * (p - 1) ** 3 + s * (p - 1) ** 2;
}

/**
 * A knock: a damped wobble starting at full swing on `at` and exactly 0
 * from `at + dur`, `freq` swings a second.
 */
export function jolt(t: number, at: number, dur: number, freq = 6): number {
  const p = progress(at, at + dur, t);
  if (p <= 0 || p >= 1) return 0;
  return Math.sin(TAU * freq * (t - at)) * (1 - p) ** 2;
}

/** A bump up from 0 and back to exactly 0: sin² over [at, at + dur]. */
export function bump(t: number, at: number, dur: number): number {
  const p = progress(at, at + dur, t);
  return p <= 0 || p >= 1 ? 0 : Math.sin(Math.PI * p) ** 2;
}
