// open's pen and line (storyboard §6.1): the light a pen carries as it inks
// the wordmark stroke by stroke and hops between strokes, the white-hot
// stretch of fresh ink cooling behind it, the dotted guide it traces, and
// the fishing line, taut and then whipping away once the hook has docked.
// The marks themselves are the kit's (kit/logo.ts); this file draws only
// what lights them. Every function is a pure function of its arguments.

import { PALETTE } from "../bible";
import { rgba } from "../color";
import { glow } from "../fx";
import { LOGO_CENTER, LOGO_STROKES, type LogoPlace, logoScale, logoToPx, STROKE, strokeTip, SWING_PIVOT } from "../kit/logo";
import { arc, curveAt, type Pt } from "../kit/motion";
import { clamp, DEG, inOutSine, lerp, progress } from "../math";

// Path2D exists only in a browser, so the paths are built on first use.
let paths: Path2D[] | null = null;
const strokePath = (i: number): Path2D => (paths ??= LOGO_STROKES.map((s) => new Path2D(s.d)))[i];

/** The leg and the barb turn with the swing, about SWING_PIVOT. */
const swings = (i: number): boolean => i === STROKE.kLeg || i === STROKE.kBarb;

/** Into `place`'s logo units, multiplied onto the caller's transform (never set). */
function intoLogo(ctx: CanvasRenderingContext2D, place: LogoPlace, swing = 0): void {
  const s = logoScale(place);
  ctx.transform(s, 0, 0, s, place.cx - LOGO_CENTER[0] * s, place.cy - LOGO_CENTER[1] * s);
  if (swing) {
    const [x, y] = SWING_PIVOT;
    ctx.translate(x, y);
    ctx.rotate(swing * DEG);
    ctx.translate(-x, -y);
  }
}

/**
 * The wordmark's centre lines as a dotted guide, one round dot every
 * 4 units: the path the pen is about to trace. Drawn under the ink, which
 * covers it exactly as each stroke lands.
 */
export function drawGuide(ctx: CanvasRenderingContext2D, place: LogoPlace, alpha: number): void {
  if (!(alpha > 0)) return;
  ctx.save();
  intoLogo(ctx, place);
  ctx.strokeStyle = rgba(PALETTE.logo, alpha);
  ctx.lineWidth = 1.7;
  ctx.lineCap = "round";
  ctx.setLineDash([0.001, 4]);
  for (let i = 0; i < LOGO_STROKES.length; i++) ctx.stroke(strokePath(i));
  ctx.restore();
}

/**
 * The fresh end of stroke `i` drawn to progress `p`: a white-hot core over
 * the last `tail` units behind the pen, brightest at the tip, at `heat`.
 */
export function drawHotInk(
  ctx: CanvasRenderingContext2D,
  place: LogoPlace,
  i: number,
  p: number,
  heat: number,
  swing = 0,
  tail = 16,
): void {
  if (!(heat > 0) || !(p > 0)) return;
  const len = LOGO_STROKES[i].len;
  const at = clamp(p) * len;
  const n = 12;
  ctx.save();
  intoLogo(ctx, place, swings(i) ? swing : 0);
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.lineWidth = 4.2;
  // Nested stretches, each a little brighter where they overlap, so the
  // heat falls off behind the tip without steps.
  ctx.strokeStyle = rgba(PALETTE.glint, 0.09 * clamp(heat));
  for (let k = 1; k <= n; k++) {
    const from = Math.max(0, at - (tail * k) / n);
    if (at - from < 0.05) continue;
    ctx.setLineDash([at - from, len + 40]);
    ctx.lineDashOffset = -from;
    ctx.stroke(strokePath(i));
  }
  ctx.restore();
}

/** A stroke's window, seconds. */
export type Window = readonly [start: number, end: number];

export interface PenState {
  /** Where the tip is, px. */
  at: Pt;
  /** 1 while it inks a stroke, dimmer in the air between strokes. */
  ink: number;
}

/**
 * The pen's tip at `lt`: on stroke i at `progressOf(i)` inside its window,
 * or hopping through the air, eased, from one stroke's end to the next
 * one's start. Before the first window it waits at the first stroke's start;
 * after the last it rests where it finished.
 */
export function penAt(
  lt: number,
  place: LogoPlace,
  windows: readonly Window[],
  progressOf: (i: number) => number,
  swing = 0,
): PenState {
  const px = (i: number, p: number): Pt => {
    const q = strokeTip(i, p);
    return logoToPx(place, q.x, q.y, swings(i) ? swing : 0);
  };
  if (lt <= windows[0][0]) return { at: px(0, 0), ink: 1 };
  for (let i = 0; i < windows.length; i++) {
    const b = windows[i][1];
    if (lt <= b) return { at: px(i, progressOf(i)), ink: 1 };
    const next = windows[i + 1];
    if (!next) break;
    if (lt < next[0]) {
      const u = progress(b, next[0], lt);
      const hop = arc(px(i, 1), px(i + 1, 0), 0.35);
      // Lifts off and sets down softly: dimmest at the top of the hop.
      return { at: curveAt(hop, inOutSine(u)), ink: lerp(1, 0.42, Math.sin(Math.PI * u)) };
    }
  }
  const last = windows.length - 1;
  return { at: px(last, 1), ink: 1 };
}

/** The pen's light: a soft logo-cyan glow with a white core, `r` px across its halo. */
export function drawPenLight(ctx: CanvasRenderingContext2D, at: Pt, ink: number, alpha: number, r = 58): void {
  const a = clamp(alpha);
  if (!(a > 0)) return;
  const size = r * lerp(0.7, 1, ink);
  glow(ctx, at.x, at.y, size, PALETTE.logo, 0.55 * a * lerp(0.6, 1, ink));
  glow(ctx, at.x, at.y, size * 0.3, PALETTE.glint, 0.95 * a * lerp(0.5, 1, ink));
}

/**
 * The fishing line from `topY` down to the hook's eye at `endY`, and its
 * whip once the hook lets go of it as `u` runs 0 to 1: the free end springs
 * up and out past the top of the frame, the line bending into an S that
 * travels with it. At `u` 0 it is exactly drawIconHook's `line` (2 px logo
 * cyan at 0.7, straight up from the eye), which the scene draws itself so
 * the hook's halo does not light it.
 */
export function drawWhipLine(ctx: CanvasRenderingContext2D, x: number, topY: number, endY: number, u: number): void {
  if (!(u < 1)) return;
  const k = clamp(u);
  // The end accelerates away (easeIn), drifting right as it curls.
  const ey = lerp(endY, topY - 60, k * k * k);
  const ex = x + 46 * Math.sin(Math.PI * k) * (1 - 0.4 * k);
  const len = ey - topY;
  if (len <= 0) return;
  // The bend grows with the slack and shrinks with what is left of the line.
  const bow = 90 * Math.sin(Math.PI * Math.min(1, k * 1.25)) * (len / Math.max(1, endY - topY));
  ctx.save();
  ctx.strokeStyle = rgba(PALETTE.logo, 0.7);
  ctx.lineWidth = 2;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(x, topY);
  ctx.bezierCurveTo(x + bow, topY + len / 3, ex - bow * 0.8, topY + (2 * len) / 3, ex, ey);
  ctx.stroke();
  ctx.restore();
}
