// K1: hk's marks (storyboard §4 K1, identity §1). The wordmark is
// docs/public/logo.svg's five paths, split into the six strokes a pen draws
// (the arm reversed so it grows out of the stem, the barb split from the leg
// because canvas restarts a dash on every subpath); the icon hook is the
// favicon's hook rebuilt in the same units, sharing the k's bowl, so it can
// dock into the wordmark. The glint and sparkle are the hook's highlights,
// and swingDeg its pendulum. Lengths are hard-coded, so a frame never
// measures a path; logo.test.ts checks them against Chromium's.
//
// Everything is placed with ctx.transform, multiplied onto the caller's
// transform, never set: the reel draws at any output size.

import { PALETTE } from "../bible";
import { rgba } from "../color";
import { glow, makeCanvas } from "../fx";
import { clamp, DEG, lerp, smoothstep, TAU } from "../math";
import { BEAT } from "../timeline";
import type { Pt } from "./motion";

/** The wordmark in logo.svg units (viewBox 10 4 140 112), in draw-on order. */
export const LOGO_STROKES = [
  { id: "hStem", d: "M28 16 V80", len: 64 },
  { id: "hShoulder", d: "M28 58 A16 16 0 0 1 60 58 V80", len: 72.27 },
  { id: "kStem", d: "M92 16 V80", len: 64 },
  // Reversed from the SVG, so it grows out of the stem.
  { id: "kArm", d: "M92 68 L126 46", len: 40.5 },
  { id: "kLeg", d: "M100 62 L122 76 C130 81 132 87 132 92 A13 13 0 0 1 106 92 V86", len: 92.69 },
  { id: "kBarb", d: "M106 86 L112 91", len: 7.81 },
] as const;

export type LogoStrokeId = (typeof LOGO_STROKES)[number]["id"];

/** Each stroke's index in LOGO_STROKES and in drawLogo's progress array. */
export const STROKE: Readonly<Record<LogoStrokeId, number>> = {
  hStem: 0,
  hShoulder: 1,
  kStem: 2,
  kArm: 3,
  kLeg: 4,
  kBarb: 5,
};

/** Every stroke drawn. */
export const LOGO_FULL: readonly number[] = [1, 1, 1, 1, 1, 1];
/** The one stroke width, logo units, round caps and joins. */
export const LOGO_STROKE = 10;
/** The optical centre, which is the viewBox centre. */
export const LOGO_CENTER = [80, 60] as const;
export const LOGO_BOX = { x: 10, y: 4, w: 140, h: 112 } as const;
/** The ink's bounds with the half stroke. */
export const LOGO_INK = { x0: 23, y0: 11, x1: 137, y1: 110 } as const;
/** The leg's root, under the arm: the hook swings about it and the join never opens. */
export const SWING_PIVOT = [100, 63] as const;
/** The hook's point, where the barb flicks from and the sparkle lands. */
export const LOGO_POINT = [106, 86] as const;
/** The leg's straight run, `M100 62 L122 76`, ≈ 26.08 units: the leg's progress there is KLEG_RUN / its length. */
export const KLEG_RUN = Math.hypot(22, 14);

/** Where a mark sits: its viewBox centre at (cx, cy), the viewBox `h` px tall. */
export interface LogoPlace {
  cx: number;
  cy: number;
  h: number;
}

/** The wordmark in `open`, 2.5 px a unit: ink x 817.5–1102.5, y 277.5–525; 25 px strokes. */
export const LOGO_OPEN: LogoPlace = { cx: 960, cy: 400, h: 280 };
/** The wordmark on the end card, 3.93 px a unit: ink x 1196–1644, y 137.5–526.4; 39.3 px strokes. */
export const LOGO_END: LogoPlace = { cx: 1420, cy: 330, h: 440 };

/** Pixels per logo unit at a place. */
export const logoScale = (place: LogoPlace): number => place.h / LOGO_BOX.h;

/**
 * A logo-unit point in px at `place`. `swing` (degrees) turns it about
 * SWING_PIVOT, as drawLogo turns the leg and barb, so a sparkle can ride
 * the swinging point.
 */
export function logoToPx(place: LogoPlace, x: number, y: number, swing = 0): Pt {
  if (swing) [x, y] = turn(x, y, SWING_PIVOT[0], SWING_PIVOT[1], swing);
  const s = logoScale(place);
  return { x: place.cx + (x - LOGO_CENTER[0]) * s, y: place.cy + (y - LOGO_CENTER[1]) * s };
}

/**
 * The draw-on at a constant pen speed with a `lift` of air between strokes:
 * each stroke's progress when the pen is `u` of the way (identity §1.3). A
 * scene on the beat eases each stroke in its own window instead; this is
 * the even-handed default.
 */
export function penStrokes(u: number, lift = 10): number[] {
  if (u >= 1) return [...LOGO_FULL];
  const total = LOGO_STROKES.reduce((s, k) => s + k.len, 0) + lift * (LOGO_STROKES.length - 1);
  let at = Math.max(0, u) * total;
  return LOGO_STROKES.map(({ len }) => {
    const p = clamp(at / len);
    at -= len + lift;
    return p;
  });
}

/** The pen's tip on stroke `i` at progress `p`, logo units: where a pen-tip glow goes. */
export function strokeTip(i: number, p: number): Pt {
  return pointAt(geometry()[i], clamp(p));
}

/** A path's length, measured from its segments: the hard-coded lengths' check. */
export function pathLength(d: string): number {
  return measure(d).len;
}

export interface LogoOptions {
  /** Default logo cyan. Keep it opaque and fade with `alpha`, which composites the mark as one. */
  color?: string;
  /** Group opacity: overlapping strokes do not darken where they cross. */
  alpha?: number;
  /** Degrees, positive clockwise: the leg and barb turned about SWING_PIVOT. */
  swing?: number;
}

// Path2D exists only in a browser, so the paths are built on first use.
let logoPaths: Path2D[] | null = null;
let fullLegAndBarb: Path2D | null = null;

/** The barb drawn to `p`, as the whole leg's last subpath or on its own. */
function barbPath(p: number, withLeg: boolean): Path2D {
  const [x, y] = LOGO_POINT;
  const tip = `M${x} ${y} L${x + 6 * Math.min(1, p)} ${y + 5 * Math.min(1, p)}`;
  if (!withLeg) return new Path2D(tip);
  if (p >= 1) return (fullLegAndBarb ??= new Path2D(`${LOGO_STROKES[STROKE.kLeg].d} ${tip}`));
  return new Path2D(`${LOGO_STROKES[STROKE.kLeg].d} ${tip}`);
}

/**
 * The wordmark at `place`, each stroke drawn to its progress in `p` (six
 * values, 0..1, in LOGO_STROKES order). A stroke at 0 is skipped, because a
 * zero-length dash with round caps still paints a dot.
 */
export function drawLogo(ctx: CanvasRenderingContext2D, place: LogoPlace, p: readonly number[], o: LogoOptions = {}): void {
  const { color = PALETTE.logo, alpha = 1, swing = 0 } = o;
  if (!(alpha > 0) || !LOGO_STROKES.some((_, i) => p[i] > 0)) return;
  logoPaths ??= LOGO_STROKES.map((s) => new Path2D(s.d));
  const paths = logoPaths;
  const leg = p[STROKE.kLeg];
  const barb = p[STROKE.kBarb];
  const paint = (c: CanvasRenderingContext2D) => {
    c.save();
    placeLogo(c, place);
    c.strokeStyle = color;
    c.lineWidth = LOGO_STROKE;
    c.lineCap = "round";
    c.lineJoin = "round";
    for (let i = 0; i < STROKE.kLeg; i++) {
      const v = p[i];
      if (!(v > 0)) continue;
      // The gap outruns the stroke, so no second dash ever starts.
      c.setLineDash(v < 1 ? [v * LOGO_STROKES[i].len, LOGO_STROKES[i].len + 20] : []);
      c.stroke(paths[i]);
    }
    if (leg > 0 || barb > 0) {
      if (swing) turnAbout(c, SWING_PIVOT, swing);
      // The barb's cap and the point's are one circle. Stroked apart, its
      // antialiased rim is laid down twice and the point looks heavier than
      // logo.svg's, which strokes leg and barb as one path; so a finished leg
      // takes the barb into its own stroke. The barb is straight, so its dash
      // is the shorter segment.
      const withBarb = leg >= 1 && barb > 0;
      if (leg > 0) {
        c.setLineDash(leg < 1 ? [leg * LOGO_STROKES[STROKE.kLeg].len, LOGO_STROKES[STROKE.kLeg].len + 20] : []);
        c.stroke(withBarb ? barbPath(barb, true) : paths[STROKE.kLeg]);
      }
      if (barb > 0 && !withBarb) {
        c.setLineDash([]);
        c.stroke(barbPath(barb, false));
      }
    }
    c.restore();
  };
  // The leg reaches at most 60 units from the pivot, so this box holds any swing.
  const box = [logoToPx(place, 8, 2), logoToPx(place, 162, 2), logoToPx(place, 162, 125), logoToPx(place, 8, 125)];
  withAlpha(ctx, alpha, box, paint);
}

/** The favicon hook in logo units: it shares the k's bowl (centre 119, 92, r 13). */
export const ICON_HOOK = {
  /**
   * From under the eye, down the shank, round the bowl and up to the point.
   * drawIconHook starts the shank where its round cap clears the eye's hole
   * (y 73.55 at 6.5 wide), as the favicon's does; the ring hides the start.
   */
  shank: "M132 73 V92 A13 13 0 0 1 106 92 V84",
  barb: "M106 84 L112 89",
  /** A stroked ring. */
  eye: { cx: 132, cy: 68, r: 5 },
  /** The ring's width. */
  ring: 5.4,
  /** Thinner than the wordmark's 10. */
  stroke: 6.5,
  /** The bowl it shares with the k. */
  bowl: { cx: 119, cy: 92, r: 13 },
  /** The point's y, free (the favicon) and docked (the k). */
  point: { free: 84, docked: 86 },
} as const;

/** The fishing line tied to the hook's eye. */
export interface HookLine {
  /** Where the line starts, px: above the frame for a line from the top edge. */
  topY: number;
  /** Where it ends, px; default the top of the eye ring. Raise it to pull a slack line away. */
  bottomY?: number;
  /** A slack line bows this many px sideways at its middle (a quadratic sag); 0 is taut. */
  slack?: number;
  alpha?: number;
}

export interface IconHookOptions {
  /** px, applied after the placement: a drop, a bob, a tug. */
  offset?: readonly [number, number];
  /** Degrees, positive clockwise, about the eye. */
  rotate?: number;
  /** The fishing line: 2 px logo cyan at 0.7, straight up from the eye. */
  line?: HookLine;
  /** Default logo cyan. Keep it opaque and fade with `alpha`. */
  color?: string;
  /** Group opacity. */
  alpha?: number;
  /**
   * The eye and the shank above y 80, multiplied onto their fade with the
   * dock (default 1): open takes them out before the dock is done, so the
   * line's knot slips off as the line lets go.
   */
  eyeAlpha?: number;
  /**
   * The barb's progress. By default it retracts into the point over the
   * second half of the dock, so the wordmark's own barb can click out of
   * the point on the next downbeat (open b4).
   */
  barb?: number;
}

// The dock bends the shank below y 80 onto the part of the k's cubic below
// y 80, so a docked hook is exactly the tail of the leg: the vertical shank
// would stand up to 5 units proud of the cubic. The cubic's y rises monotonically,
// so bisection finds where it crosses 80.
const LEG_CUBIC: readonly Pt[] = [
  { x: 122, y: 76 },
  { x: 130, y: 81 },
  { x: 132, y: 87 },
  { x: 132, y: 92 },
];
const DOCK_FROM: readonly Pt[] = [
  { x: 132, y: 80 },
  { x: 132, y: 84 },
  { x: 132, y: 88 },
  { x: 132, y: 92 },
];
const DOCK_TO: readonly Pt[] = (() => {
  let lo = 0;
  let hi = 1;
  for (let i = 0; i < 60; i++) {
    const mid = (lo + hi) / 2;
    if (cubicAt(LEG_CUBIC, mid).y < 80) lo = mid;
    else hi = mid;
  }
  return splitRight(LEG_CUBIC, (lo + hi) / 2);
})();

/**
 * The favicon's hook in the logo units of `place`, `k` of the way docked
 * into the k: at 0 a free hook (stroke 6.5, point y 84, the eye at full
 * strength); at 1 the tail of the k's leg (stroke 10, point y 86, the eye
 * and the shank above y 80 gone, the rest bent onto the leg), which the
 * wordmark's leg covers exactly. With `offset` 0 its bowl sits on the k's.
 */
export function drawIconHook(ctx: CanvasRenderingContext2D, place: LogoPlace, k: number, o: IconHookOptions = {}): void {
  const alpha = o.alpha ?? 1;
  if (!(alpha > 0)) return;
  const dock = clamp(k);
  const color = o.color ?? PALETTE.logo;
  const [dx, dy] = o.offset ?? [0, 0];
  const width = lerp(ICON_HOOK.stroke, LOGO_STROKE, dock);
  const py = lerp(ICON_HOOK.point.free, ICON_HOOK.point.docked, dock);
  const barb = clamp(o.barb ?? 1 - smoothstep(0.5, 1, dock));
  const fade = (1 - dock) * clamp(o.eyeAlpha ?? 1);
  const q = DOCK_FROM.map((a, i) => ({ x: lerp(a.x, DOCK_TO[i].x, dock), y: lerp(a.y, DOCK_TO[i].y, dock) }));
  const { cx: ex, cy: ey, r: er } = ICON_HOOK.eye;

  if (o.line) drawLine(ctx, place, o.line, dx, dy, color, alpha);

  const paint = (c: CanvasRenderingContext2D) => {
    c.save();
    c.translate(dx, dy);
    placeLogo(c, place);
    if (o.rotate) turnAbout(c, [ex, ey], o.rotate);
    c.strokeStyle = color;
    c.lineCap = "round";
    c.lineJoin = "round";
    // The eye and the shank above y 80 fade together, as one layer, so they
    // do not double up where the shank meets the ring.
    const top = (l: CanvasRenderingContext2D) => {
      l.strokeStyle = color;
      l.lineCap = "round";
      l.lineWidth = width;
      l.beginPath();
      l.moveTo(ex, ey + er - ICON_HOOK.ring / 2 + width / 2);
      l.lineTo(q[0].x, q[0].y);
      l.stroke();
      l.lineWidth = ICON_HOOK.ring;
      l.beginPath();
      l.arc(ex, ey, er, 0, TAU);
      l.stroke();
    };
    const eyeBox = [
      { x: ex - 12, y: ey - 12 },
      { x: ex + 12, y: ey - 12 },
      { x: ex + 12, y: 90 },
      { x: ex - 12, y: 90 },
    ];
    if (fade > 0) withAlpha(c, fade, eyeBox, top);
    // Shank, bowl, point and barb: one path, so the point's cap and the
    // barb's are laid down once. The barb is straight: its progress is a
    // shorter segment.
    c.lineWidth = width;
    c.beginPath();
    c.moveTo(q[0].x, q[0].y);
    c.bezierCurveTo(q[1].x, q[1].y, q[2].x, q[2].y, q[3].x, q[3].y);
    c.arc(ICON_HOOK.bowl.cx, ICON_HOOK.bowl.cy, ICON_HOOK.bowl.r, 0, Math.PI);
    c.lineTo(106, py);
    if (barb > 0) {
      c.moveTo(106, py);
      c.lineTo(106 + 6 * barb, py + 5 * barb);
    }
    c.stroke();
    c.restore();
  };
  // The hook's ink lies within 50 units of its eye, whichever way it turns.
  const box = [
    { x: ex - 52, y: ey - 52 },
    { x: ex + 52, y: ey - 52 },
    { x: ex + 52, y: ey + 52 },
    { x: ex - 52, y: ey + 52 },
  ].map((p) => {
    const at = logoToPx(place, p.x, p.y);
    return { x: at.x + dx, y: at.y + dy };
  });
  withAlpha(ctx, alpha, box, paint);
}

/** The place that puts the icon hook's eye at (x, y), `scale` px a logo unit (catch: 3). */
export function hookPlace(x: number, y: number, scale: number): LogoPlace {
  const { cx, cy } = ICON_HOOK.eye;
  return { cx: x - (cx - LOGO_CENTER[0]) * scale, cy: y - (cy - LOGO_CENTER[1]) * scale, h: LOGO_BOX.h * scale };
}

/** Where the icon hook's parts are, px, with the same `k`, offset and rotation as drawIconHook. */
export interface HookAnchors {
  /** The eye's centre. */
  eye: Pt;
  /** The top of the eye ring's ink, where the line is tied. */
  ringTop: Pt;
  /** The bowl's centre. */
  bowl: Pt;
  /** The bottom of the bowl, on the stroke's centre line: what a loop hangs from. */
  bottom: Pt;
  /** The point. */
  point: Pt;
}

export function hookAnchors(place: LogoPlace, k = 0, o: Pick<IconHookOptions, "offset" | "rotate"> = {}): HookAnchors {
  const { cx: ex, cy: ey, r: er } = ICON_HOOK.eye;
  const { cx: bx, cy: by, r: br } = ICON_HOOK.bowl;
  const [dx, dy] = o.offset ?? [0, 0];
  const at = (x: number, y: number): Pt => {
    if (o.rotate) [x, y] = turn(x, y, ex, ey, o.rotate);
    const p = logoToPx(place, x, y);
    return { x: p.x + dx, y: p.y + dy };
  };
  const eye = at(ex, ey);
  return {
    eye,
    // The ring is round, so its top is straight above the eye however it turns.
    ringTop: { x: eye.x, y: eye.y - (er + ICON_HOOK.ring / 2) * logoScale(place) },
    bowl: at(bx, by),
    bottom: at(bx, by + br),
    point: at(106, lerp(ICON_HOOK.point.free, ICON_HOOK.point.docked, clamp(k))),
  };
}

/**
 * A highlight running along the leg as `u` goes 0 to 1: a 9-unit dash,
 * 3.2 units wide, `#ecfdff` at 0.9, whose offset runs from 0 to minus the
 * leg's length, so it runs off the point. `swing` turns it with the leg.
 */
export function glint(ctx: CanvasRenderingContext2D, place: LogoPlace, u: number, o: { swing?: number; alpha?: number } = {}): void {
  // At 1 the dash has run off the point; a zero-length dash would still paint a cap there.
  if (!(u > 0 && u < 1)) return;
  const alpha = 0.9 * (o.alpha ?? 1);
  if (!(alpha > 0)) return;
  logoPaths ??= LOGO_STROKES.map((s) => new Path2D(s.d));
  const leg = LOGO_STROKES[STROKE.kLeg];
  ctx.save();
  placeLogo(ctx, place);
  // The leg's first units lie under the arm, so near the root the glint is
  // clipped to outside the arm: it comes out from under it, as the leg does.
  if (u * leg.len < 12) {
    const [a, b] = [
      { x: 92, y: 68 },
      { x: 126, y: 46 },
    ];
    const th = Math.atan2(b.y - a.y, b.x - a.x);
    const r = LOGO_STROKE / 2;
    ctx.beginPath();
    ctx.rect(LOGO_BOX.x, LOGO_BOX.y, LOGO_BOX.w, LOGO_BOX.h);
    ctx.moveTo(b.x + r * Math.cos(th - Math.PI / 2), b.y + r * Math.sin(th - Math.PI / 2));
    ctx.arc(b.x, b.y, r, th - Math.PI / 2, th + Math.PI / 2);
    ctx.arc(a.x, a.y, r, th + Math.PI / 2, th + (3 * Math.PI) / 2);
    ctx.closePath();
    ctx.clip("evenodd");
  }
  if (o.swing) turnAbout(ctx, SWING_PIVOT, o.swing);
  ctx.strokeStyle = rgba(PALETTE.glint, alpha);
  ctx.lineWidth = 3.2;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.setLineDash([9, 400]);
  ctx.lineDashOffset = -u * leg.len;
  ctx.stroke(logoPaths[STROKE.kLeg]);
  ctx.restore();
}

/**
 * A four-point star at (x, y) px: its radius rises from 0 to 22 px at
 * `u` = 0.5 and back to 0 at 1, turning a little as it goes.
 */
export function sparkle(ctx: CanvasRenderingContext2D, x: number, y: number, u: number): void {
  if (!(u > 0 && u < 1)) return;
  const r = 22 * Math.sin(Math.PI * u);
  glow(ctx, x, y, r * 1.9, PALETTE.glint, 0.5);
  const c = r * 0.14;
  ctx.save();
  ctx.translate(x, y);
  ctx.rotate((u - 0.5) * 0.5);
  ctx.fillStyle = PALETTE.glint;
  ctx.beginPath();
  ctx.moveTo(0, -r);
  ctx.quadraticCurveTo(c, -c, r, 0);
  ctx.quadraticCurveTo(c, c, 0, r);
  ctx.quadraticCurveTo(-c, c, -r, 0);
  ctx.quadraticCurveTo(-c, -c, 0, -r);
  ctx.fill();
  ctx.restore();
}

/**
 * A damped pendulum on the beat, in degrees: `A·e^(−(t−t0)/tau)·sin(2π(t−t0)/BEAT)`
 * from `t0`, 0 before it. Seconds throughout. It never quite stops, so
 * multiply it by an envelope that reaches 0 before a handoff (settledSwing).
 */
export function swingDeg(t: number, t0: number, A: number, tau: number): number {
  const d = t - t0;
  if (d <= 0) return 0;
  return A * Math.exp(-d / tau) * Math.sin((TAU * d) / BEAT);
}

/**
 * swingDeg eased out over `calm` ([from, to], seconds) and exactly 0 from
 * `to`: open's `swingDeg(t, b4, 8, BAR) × (1 − smoothstep(b5.5, b7))`.
 */
export function settledSwing(t: number, t0: number, A: number, tau: number, calm: readonly [number, number]): number {
  if (t >= calm[1]) return 0;
  return swingDeg(t, t0, A, tau) * (1 - smoothstep(calm[0], calm[1], t));
}

// Placement and turns.

function placeLogo(ctx: CanvasRenderingContext2D, place: LogoPlace): void {
  const s = logoScale(place);
  // Multiplied onto the reel's own scale, never set: the output may not be 1920 wide.
  ctx.transform(s, 0, 0, s, place.cx - LOGO_CENTER[0] * s, place.cy - LOGO_CENTER[1] * s);
}

function turnAbout(ctx: CanvasRenderingContext2D, [x, y]: readonly [number, number], deg: number): void {
  ctx.translate(x, y);
  ctx.rotate(deg * DEG);
  ctx.translate(-x, -y);
}

/** (x, y) turned `deg` clockwise (on screen) about (px, py). */
function turn(x: number, y: number, px: number, py: number, deg: number): [number, number] {
  const c = Math.cos(deg * DEG);
  const s = Math.sin(deg * DEG);
  return [px + (x - px) * c - (y - py) * s, py + (x - px) * s + (y - py) * c];
}

function drawLine(ctx: CanvasRenderingContext2D, place: LogoPlace, line: HookLine, dx: number, dy: number, color: string, alpha: number): void {
  const a = 0.7 * (line.alpha ?? 1) * alpha;
  if (!(a > 0)) return;
  const { ringTop } = hookAnchors(place, 0, { offset: [dx, dy] });
  const x = ringTop.x;
  const y0 = line.topY;
  const y1 = line.bottomY ?? ringTop.y;
  if (y1 <= y0) return;
  ctx.save();
  ctx.strokeStyle = rgba(color, a);
  ctx.lineWidth = 2;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(x, y0);
  // A quadratic's middle sits half way to its control point.
  if (line.slack) ctx.quadraticCurveTo(x + 2 * line.slack, (y0 + y1) / 2, x, y1);
  else ctx.lineTo(x, y1);
  ctx.stroke();
  ctx.restore();
}

// Group opacity. Strokes of one colour that overlap are invisible at full
// strength but double up at partial strength, so a mark below 1 is drawn
// whole into a layer and the layer composited at its alpha.

const layers: HTMLCanvasElement[] = [];
let depth = 0;

/**
 * Paint `draw` at `alpha` times the context's own, through a layer when that
 * is below 1. `box` bounds the ink in the context's current user space.
 */
function withAlpha(ctx: CanvasRenderingContext2D, alpha: number, box: readonly Pt[], draw: (c: CanvasRenderingContext2D) => void): void {
  if (alpha * ctx.globalAlpha >= 1) {
    draw(ctx);
    return;
  }
  const m = ctx.getTransform();
  const xs = box.map((p) => m.a * p.x + m.c * p.y + m.e);
  const ys = box.map((p) => m.b * p.x + m.d * p.y + m.f);
  // Whole device pixels, so the layer lands on the grid it was drawn on.
  const x0 = Math.max(0, Math.floor(Math.min(...xs)) - 2);
  const y0 = Math.max(0, Math.floor(Math.min(...ys)) - 2);
  const x1 = Math.min(ctx.canvas.width, Math.ceil(Math.max(...xs)) + 2);
  const y1 = Math.min(ctx.canvas.height, Math.ceil(Math.max(...ys)) + 2);
  const w = x1 - x0;
  const h = y1 - y0;
  if (w <= 0 || h <= 0) return;
  let layer = layers[depth];
  if (!layer || layer.width < w || layer.height < h) {
    layer = makeCanvas(Math.max(w, layer?.width ?? 0), Math.max(h, layer?.height ?? 0));
    layers[depth] = layer;
  }
  const l = layer.getContext("2d")!;
  l.setTransform(1, 0, 0, 1, 0, 0);
  l.globalAlpha = 1;
  l.globalCompositeOperation = "source-over";
  l.clearRect(0, 0, w, h);
  l.setTransform(m.a, m.b, m.c, m.d, m.e - x0, m.f - y0);
  depth++;
  try {
    draw(l);
  } finally {
    depth--;
  }
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha *= clamp(alpha);
  ctx.drawImage(layer, 0, 0, w, h, x0, y0, w, h);
  ctx.restore();
}

// Path geometry, for the pen's tip and the length check: the absolute M, L,
// H, V, C and circular A that logo.svg uses. Pure arithmetic, measured once.

type Seg =
  | { kind: "line"; a: Pt; b: Pt; len: number }
  | { kind: "cubic"; p: readonly Pt[]; cum: readonly number[]; len: number }
  | { kind: "arc"; cx: number; cy: number; r: number; a0: number; da: number; len: number };

interface Geometry {
  start: Pt;
  segs: Seg[];
  len: number;
}

let geo: Geometry[] | null = null;
const geometry = (): Geometry[] => (geo ??= LOGO_STROKES.map((s) => measure(s.d)));

function measure(d: string): Geometry {
  const tokens = d.match(/[a-zA-Z]|-?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?/g) ?? [];
  const segs: Seg[] = [];
  let i = 0;
  let cmd = "";
  let cur: Pt = { x: 0, y: 0 };
  let start: Pt | null = null;
  const num = () => {
    const v = Number(tokens[i++]);
    if (!Number.isFinite(v)) throw new Error(`bad number in path: ${d}`);
    return v;
  };
  const line = (b: Pt) => {
    segs.push({ kind: "line", a: cur, b, len: Math.hypot(b.x - cur.x, b.y - cur.y) });
    cur = b;
  };
  while (i < tokens.length) {
    if (/[a-zA-Z]/.test(tokens[i])) cmd = tokens[i++];
    switch (cmd) {
      case "M":
        cur = { x: num(), y: num() };
        start ??= cur;
        cmd = "L";
        break;
      case "L":
        line({ x: num(), y: num() });
        break;
      case "H":
        line({ x: num(), y: cur.y });
        break;
      case "V":
        line({ x: cur.x, y: num() });
        break;
      case "C": {
        const p = [cur, { x: num(), y: num() }, { x: num(), y: num() }, { x: num(), y: num() }];
        const cum = [0];
        let prev = p[0];
        for (let n = 1; n <= 512; n++) {
          const q = cubicAt(p, n / 512);
          cum.push(cum[n - 1] + Math.hypot(q.x - prev.x, q.y - prev.y));
          prev = q;
        }
        segs.push({ kind: "cubic", p, cum, len: cum[512] });
        cur = p[3];
        break;
      }
      case "A": {
        const rx = num();
        const ry = num();
        num(); // rotation: a circle has none
        const large = num();
        const sweep = num();
        const b = { x: num(), y: num() };
        if (rx !== ry) throw new Error(`only circular arcs: ${d}`);
        segs.push(arcSeg(cur, b, rx, large !== 0, sweep !== 0));
        cur = b;
        break;
      }
      default:
        throw new Error(`unsupported path command ${cmd} in ${d}`);
    }
  }
  return { start: start ?? cur, segs, len: segs.reduce((s, g) => s + g.len, 0) };
}

/** SVG's endpoint-to-centre conversion (SVG 1.1 F.6.5) for a circle. */
function arcSeg(a: Pt, b: Pt, radius: number, large: boolean, sweep: boolean): Seg {
  const hx = (a.x - b.x) / 2;
  const hy = (a.y - b.y) / 2;
  const r = Math.max(radius, Math.hypot(hx, hy));
  const k = (large === sweep ? -1 : 1) * Math.sqrt(Math.max(0, (r * r - hx * hx - hy * hy) / (hx * hx + hy * hy)));
  const cx = k * hy + (a.x + b.x) / 2;
  const cy = -k * hx + (a.y + b.y) / 2;
  const a0 = Math.atan2(a.y - cy, a.x - cx);
  let da = Math.atan2(b.y - cy, b.x - cx) - a0;
  if (sweep && da <= 0) da += TAU;
  if (!sweep && da >= 0) da -= TAU;
  return { kind: "arc", cx, cy, r, a0, da, len: r * Math.abs(da) };
}

function pointAt(g: Geometry, p: number): Pt {
  let at = p * g.len;
  for (const s of g.segs) {
    if (at <= s.len || s === g.segs[g.segs.length - 1]) {
      const f = s.len ? clamp(at / s.len) : 0;
      if (s.kind === "line") return { x: lerp(s.a.x, s.b.x, f), y: lerp(s.a.y, s.b.y, f) };
      if (s.kind === "arc") {
        const a = s.a0 + s.da * f;
        return { x: s.cx + s.r * Math.cos(a), y: s.cy + s.r * Math.sin(a) };
      }
      // Invert the arc-length table, then interpolate within the step.
      const want = f * s.len;
      let n = 1;
      while (n < s.cum.length - 1 && s.cum[n] < want) n++;
      const step = s.cum[n] - s.cum[n - 1];
      const u = (n - 1 + (step ? (want - s.cum[n - 1]) / step : 0)) / (s.cum.length - 1);
      return cubicAt(s.p, u);
    }
    at -= s.len;
  }
  return g.start;
}

function cubicAt(p: readonly Pt[], t: number): Pt {
  const v = 1 - t;
  const a = v * v * v;
  const b = 3 * v * v * t;
  const c = 3 * v * t * t;
  const e = t * t * t;
  return { x: a * p[0].x + b * p[1].x + c * p[2].x + e * p[3].x, y: a * p[0].y + b * p[1].y + c * p[2].y + e * p[3].y };
}

/** The part of a cubic from `t` to its end (de Casteljau). */
function splitRight(p: readonly Pt[], t: number): Pt[] {
  const l = (a: Pt, b: Pt): Pt => ({ x: lerp(a.x, b.x, t), y: lerp(a.y, b.y, t) });
  const p01 = l(p[0], p[1]);
  const p12 = l(p[1], p[2]);
  const p23 = l(p[2], p[3]);
  const p012 = l(p01, p12);
  const p123 = l(p12, p23);
  return [l(p012, p123), p123, p23, p[3]];
}
