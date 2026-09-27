// The stash scene's moment (storyboard §6.4 b2–3): the unstaged TODO line,
// cut off the main.py card by a blade, peels up and is flung in an arc into
// the stash tray. Everything here is a pure function of the scene's beat.
//
// The strip is paper with a thickness of nothing: a row of 2 px columns,
// each placed by where the strip's arc length has got to. On the card it
// lies flat. Peeling, the part behind the peel front bends round a small
// radius and rises at the peel angle, its free end curling tighter, as tape
// does. The front eases into the far end, where the strip hangs on a
// moment, still curling, after the blade has gone; then it is free: its
// centre of mass eases into a ballistic arc (over the ground, parabolic in
// height) while the curl relaxes with a flutter, comes to a stop over the
// tray and drops into its mouth, in front of the open lid, and shrinks and
// turns edge-on into the slot, where it becomes the tray's sliver.
//
// The camera looks down on the card from above the frame's foot, so height
// shows as a shift up the screen (LIFT px per px) and a little growth
// (PERSP), and the strip's shadow stays on the card where it was. Faces
// are shaded by how far they turn from a light above and to the left, and
// the back of the paper shows where a column turns past upright.

import { PALETTE } from "../bible";
import { mix, rgba } from "../color";
import { glow, makeCanvas, roundedRect } from "../fx";
import { drawMono, MAIN_PY_TODO } from "../kit/card";
import { clamp, cubicBezier, hash, lerp, progress, smoothstep, TAU } from "../math";
import { BEAT } from "../timeline";

// Where the strip lies on the card: row 10 of the main.py card (x 660–1260,
// y 284–696, 26 px mono on a 32 px pitch), baseline 676.8, text from x 684.

/**
 * The TODO strip at rest on the card, logical px. Its top edge is the cut,
 * 2–3 px clear of the `()` of `main()` on the row above (they reach y 650.5).
 */
export const STRIP = { x: 670, y: 653, w: 466, h: 36, r: 7 } as const;
/** The line's text from the strip's left edge, and its baseline from the top (676.8, the card's row 10). */
const TEXT_DX = 14;
const TEXT_DY = 23.8;
const SIZE = 26;
/** The dashed outline: the user's own line, marked for cutting. */
export const DASH: readonly number[] = [12, 8];
const DASH_PERIOD = 20;

/** The strip's paper: the card's surface warmed a touch, and its back a touch lighter. */
export const STRIP_BODY = mix(PALETTE.surface, PALETTE.paper, 0.14);
const STRIP_BACK = mix(PALETTE.surface, PALETTE.paper, 0.3);
/** The tray's sliver colour (tray.ts), which the strip turns into in the slot. */
const SLIVER = mix(PALETTE.paper, PALETTE.paperDim, 0.35);

// The camera.

/** Screen px up per px of height above the card. */
const LIFT = 0.45;
/** Height z draws 1 + z / PERSP as large. */
const PERSP = 1000;
/** The shadow falls a little right and down of the strip: px per px of height. */
const SHADOW_DX = 0.16;
const SHADOW_DY = 0.05;

/** Column width, logical px along the strip. */
const COL = 2;
const COLS = Math.round(STRIP.w / COL);

// Timing, in scene beats (storyboard §6.4: slice on b2, lid shut on b3).

/** The blade enters, runs along the strip's top edge (y 653), and leaves the card in a sixteenth. */
export const CUT = { from: 2, to: 2.25, x0: 628, x1: 1310, y: STRIP.y } as const;
/** The card's edges, where the blade starts cutting and throws its sparks. */
const CARD_X0 = 660;
const CARD_X1 = 1260;
/**
 * The peel front runs from the free end to the far end behind the blade and
 * eases into it; the strip goes on curling up from its far end for a moment
 * after the blade has gone, then lets go.
 */
export const PEEL = { from: 2.0625, to: 2.375 } as const;
/** The free strip's flight, from the release to the slot. */
export const FLY = { from: PEEL.to, to: 2.9375 } as const;
/** It is gone into the tray as the lid slams on b3. */
export const INTO = { from: FLY.to, to: 3 } as const;

/** Bend radius at the peel front, px. */
const BEND_R = 12;
/** The peel angle, rising as the front runs. */
const PHI0 = 0.22;
const PHI1 = 0.34;
/** The lifted tail's gentle curvature, rad per px. */
const TAIL_K = 0.00055;
/** The free end's own curl: rad at the tip, fading over CURL_LEN px. */
const CURL = 1.55;
const CURL_LEN = 84;

/** Where the flight ends: the strip level in the tray's slot as the sliver (tray.ts, x 1540–1740, from y 626). */
const SLOT = { cx: 1640, top: 626, w: 200, h: 6.6 } as const;
const SCALE_END = SLOT.w / STRIP.w;
const SQUASH_END = SLOT.h / (STRIP.h * SCALE_END);
/** How high the arc rises over its ground track, px of height. */
const ARC_H = 260;

const bladeEase = cubicBezier(0.5, 0, 0.45, 1);
/** The blade's x at beat b. */
export const bladeX = (b: number): number => lerp(CUT.x0, CUT.x1, bladeEase(progress(CUT.from, CUT.to, b)));

/** The beat the blade crosses x (inverting bladeX by bisection). */
export function bladeAt(x: number): number {
  let lo: number = CUT.from;
  let hi: number = CUT.to;
  for (let i = 0; i < 30; i++) {
    const m = (lo + hi) / 2;
    if (bladeX(m) < x) lo = m;
    else hi = m;
  }
  return (lo + hi) / 2;
}

/** The beat the blade leaves the card's right edge and throws its sparks. */
export const SPARKS_AT = bladeAt(CARD_X1);

// The strip's face.

/**
 * The strip's front: its paper, the dashed outline marching by `dash` px,
 * and the TODO line in paper. At (x, y), its top-left corner.
 */
export function drawStripFace(ctx: CanvasRenderingContext2D, x: number, y: number, o: { dash?: number; body?: string } = {}): void {
  const { w, h, r } = STRIP;
  ctx.save();
  roundedRect(ctx, x, y, w, h, r);
  ctx.fillStyle = o.body ?? STRIP_BODY;
  ctx.fill();
  roundedRect(ctx, x + 1, y + 1, w - 2, h - 2, r - 1);
  ctx.setLineDash([...DASH]);
  ctx.lineDashOffset = -(o.dash ?? 0);
  ctx.strokeStyle = PALETTE.paper;
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.restore();
  drawMono(ctx, MAIN_PY_TODO, x + TEXT_DX, y + TEXT_DY, SIZE, PALETTE.paper);
}

/** The outline's march: two periods from b1 to b2, so it is back in phase, and still, when the blade comes. */
export const dashAt = (b: number): number => DASH_PERIOD * 2 * progress(1, 2, b);

// The strip as a row of columns.

interface Sample {
  /** World x, px. */
  x: number;
  /** Height above the card, px. */
  z: number;
  /** The face's angle from lying flat, rad: past π/2 its back shows. */
  th: number;
}

/**
 * The peel's shape: the angle at arc length s (from the free end) when the
 * front is at p, with the peel angle phi and the tip's curl.
 */
function peelAngle(s: number, p: number, phi: number, curl: number): number {
  const d = p - s;
  if (d <= 0) return 0;
  // Round the front's small radius up to the peel angle, then a long gentle
  // arc as the strip's own stiffness carries it on.
  const bend = Math.min(d / BEND_R, phi) + TAIL_K * d;
  const tip = s < CURL_LEN ? curl * (1 - s / CURL_LEN) ** 2 * smoothstep(0, 2 * BEND_R, d) : 0;
  return bend + tip;
}

/**
 * Samples at the column edges, from the free end (s = 0) to the far end
 * (s = w), given the angle along the strip. Integrated from `anchor` (an
 * arc length whose world point is `at`) outward both ways; the part past the
 * anchor toward the far end is taken as lying at the anchor's angle 0.
 */
function integrate(angle: (s: number) => number, anchor: number, at: { x: number; z: number }): Sample[] {
  const out: Sample[] = new Array(COLS + 1);
  const a = clamp(anchor, 0, STRIP.w);
  // Flat from the anchor to the far end.
  for (let j = COLS; j >= 0; j--) {
    const s = j * COL;
    if (s < a) break;
    out[j] = { x: at.x + (s - a), z: at.z, th: 0 };
  }
  // Bent from the anchor back to the free end.
  let x = at.x;
  let z = at.z;
  let s = a;
  for (let j = Math.ceil(a / COL - 1e-9) - 1; j >= 0; j--) {
    const s1 = j * COL;
    // A sub-step from s back to s1, at its middle's angle.
    const th = angle((s + s1) / 2);
    const ds = s - s1;
    x -= Math.cos(th) * ds;
    z += Math.sin(th) * ds;
    s = s1;
    out[j] = { x, z, th: angle(s1) };
  }
  return out;
}

/** Where the strip is and how it is held: its columns, and the screen transform over them. */
export interface StripPose {
  samples: Sample[];
  /** The ground line under the strip's middle, px. */
  yc: number;
  /** Screen transform about a pivot: in-plane tilt (rad), scale, and a vertical squash. */
  pivot: { x: number; y: number };
  tilt: number;
  scale: number;
  squash: number;
  /** 0..1 the paper turning the sliver's colour. */
  sliver: number;
  /** How much of its shadow falls, 0..1. */
  shadow: number;
  alpha: number;
}

/** The strip lying on the card. */
const FLAT: readonly Sample[] = Array.from({ length: COLS + 1 }, (_, j) => ({ x: STRIP.x + j * COL, z: 0, th: 0 }));

/** How far through the peel the front reaches the far end. */
const FRONT_END = 0.8;

/**
 * The peel front's arc length at beat b: never ahead of the blade, quick
 * behind it, and easing into the far end, where the strip holds on a moment.
 */
function frontAt(b: number): number {
  // A cubic from 0.3 of its mean speed at the free end to rest at the far end.
  const u = Math.min(1, progress(PEEL.from, PEEL.to, b) / FRONT_END);
  const run = STRIP.w * u * (0.3 + u * (2.4 - 1.7 * u));
  return clamp(Math.min(run, bladeX(b) - STRIP.x - 24), 0, STRIP.w);
}

/** The peel angle and the tip's curl at beat b, both still growing after the blade has gone. */
function peelParams(b: number): { phi: number; curl: number } {
  const u = progress(PEEL.from, PEEL.to, b);
  return { phi: lerp(PHI0, PHI1, smoothstep(0, 1, u)), curl: CURL * smoothstep(0, 0.75, u) };
}

/** The strip's shape the moment it comes free, relative to its far end. */
function releaseAngle(s: number): number {
  const { phi, curl } = peelParams(PEEL.to);
  return peelAngle(s, STRIP.w, phi, curl);
}

/** Mean position of the samples, the strip's centre of mass (it is uniform). */
function centroid(samples: readonly Sample[]): { x: number; z: number } {
  let x = 0;
  let z = 0;
  for (let j = 0; j < samples.length - 1; j++) {
    x += (samples[j].x + samples[j + 1].x) / 2;
    z += (samples[j].z + samples[j + 1].z) / 2;
  }
  const n = samples.length - 1;
  return { x: x / n, z: z / n };
}

const RELEASE = integrate(releaseAngle, STRIP.w, { x: STRIP.x + STRIP.w, z: 0 });
const RELEASE_C = centroid(RELEASE);
const GROUND0 = { x: RELEASE_C.x, y: STRIP.y + STRIP.h / 2 };
const GROUND1 = { x: SLOT.cx, y: SLOT.top + SLOT.h / 2 };

/**
 * Height of the flying strip's centre at flight progress u: from where it
 * came free, over the arc, and down faster at the end, so it is at the
 * slot's mouth before it turns edge-on into it.
 */
const heightAt = (u: number): number => (RELEASE_C.z * (1 - u) + 4 * ARC_H * u * (1 - u)) * (1 - smoothstep(0.7, 1, u));

/**
 * How far over the ground the strip has got at flight progress u: steady,
 * then slowing to a stop over the slot at u = OVER_SLOT, so it drops into
 * the tray's mouth from above rather than sliding in through its side.
 */
const OVER_SLOT = 0.92;
const TRACK_EASE = 0.75;
function groundAt(u: number): number {
  const v = Math.min(1, u / OVER_SLOT);
  const c = 1 / (TRACK_EASE + (1 - TRACK_EASE) / 2);
  return v < TRACK_EASE ? c * v : c * (v - (v - TRACK_EASE) ** 2 / (2 * (1 - TRACK_EASE)));
}

/** The flying strip's centre on screen at flight progress u. */
function screenAt(u: number): { x: number; y: number } {
  const g = groundAt(u);
  return { x: lerp(GROUND0.x, GROUND1.x, g), y: lerp(GROUND0.y, GROUND1.y, g) - LIFT * heightAt(u) };
}

/** The start of the flight eases in over its first FLY_EASE, from FLY_K of its speed. */
const FLY_EASE = 0.15;
const FLY_K = 0.25;
const FLY_C = 1 / (1 - ((1 - FLY_K) * FLY_EASE) / 2);
/** Flight progress at tf: easing out of the peel, then flung, slowing a little as the tray takes it. */
function flightAt(tf: number): number {
  const e = tf < FLY_EASE ? FLY_C * (FLY_K * tf + ((1 - FLY_K) * tf * tf) / (2 * FLY_EASE)) : FLY_C * (tf - ((1 - FLY_K) * FLY_EASE) / 2);
  return 1 - (1 - clamp(e)) ** 1.2;
}

/** The strip's pose at beat b, or null before the cut has freed any of it (it is then part of the card) or once it is in the tray. */
export function stripPose(b: number): StripPose | null {
  if (b < PEEL.from || b >= INTO.to) return null;
  if (b < PEEL.to) {
    const p = frontAt(b);
    const { phi, curl } = peelParams(b);
    const samples = p <= 0 ? [...FLAT] : integrate((s) => peelAngle(s, p, phi, curl), p, { x: STRIP.x + p, z: 0 });
    return { samples, yc: GROUND0.y, pivot: { x: 0, y: 0 }, tilt: 0, scale: 1, squash: 1, sliver: 0, shadow: 1, alpha: 1 };
  }
  // Free. The curl lets go with a flutter and is flat by the slot.
  // Flung, it slows a little as the tray takes it; the bend it came off
  // with springs out over a few frames and flutters once.
  const tf = progress(FLY.from, FLY.to, b);
  const u = flightAt(tf);
  const relax = Math.exp(-3 * tf) * Math.cos(TAU * 0.9 * tf) * (1 - smoothstep(0.65, 0.95, tf));
  const local = integrate((s) => releaseAngle(s) * relax, STRIP.w, { x: 0, z: 0 });
  const c = centroid(local);
  const g = groundAt(u);
  const gx = lerp(GROUND0.x, GROUND1.x, g);
  const gy = lerp(GROUND0.y, GROUND1.y, g);
  const z = heightAt(u);
  const samples = local.map((q) => ({ x: q.x - c.x + gx, z: q.z - c.z + z, th: q.th }));
  // Leaning into its path, then level as it lines up with the slot.
  const du = 1e-3;
  const s0 = screenAt(Math.max(0, u - du));
  const s1 = screenAt(Math.min(1, u + du));
  const vx = s1.x - s0.x;
  const vy = s1.y - s0.y;
  const tilt = Math.atan2(vy, vx) * 0.25 * smoothstep(0, 0.3, u) * (1 - smoothstep(0.68, 0.93, u));
  return {
    samples,
    yc: gy,
    pivot: { x: gx, y: gy - LIFT * z },
    tilt,
    scale: lerp(1, SCALE_END, smoothstep(0.45, 1, u)),
    // Paper until it reaches the slot's mouth, then edge-on into it.
    squash: lerp(1, SQUASH_END, smoothstep(0.9, 1, u)),
    sliver: smoothstep(0.9, 1, u),
    shadow: 1 - smoothstep(0.45, 0.8, u),
    alpha: 1 - progress(INTO.from, INTO.to, b),
  };
}

// Drawing it.

let face: HTMLCanvasElement | null = null;
let shade: HTMLCanvasElement | null = null;
/** The strip's face is drawn into this with a margin, at the frame's device scale. */
const PAD = 4;

/** Device px per logical px of the frame's current transform. */
const deviceScale = (ctx: CanvasRenderingContext2D): number => {
  const m = ctx.getTransform();
  return Math.hypot(m.a, m.b) || 1;
};

/** The face at device scale `d`, redrawn each frame (it is small), tinted `sliver` of the way to the tray's sliver. */
function faceTexture(d: number, sliver: number): HTMLCanvasElement {
  const w = Math.ceil((STRIP.w + 2 * PAD) * d);
  const h = Math.ceil((STRIP.h + 2 * PAD) * d);
  if (!face || face.width !== w || face.height !== h) face = makeCanvas(w, h);
  const g = face.getContext("2d")!;
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.globalAlpha = 1;
  g.globalCompositeOperation = "source-over";
  g.clearRect(0, 0, w, h);
  g.setTransform(d, 0, 0, d, PAD * d, PAD * d);
  drawStripFace(g, 0, 0);
  if (sliver > 0) {
    // Into the slot it reads as the tray's sliver: its paper, edge-on.
    g.globalCompositeOperation = "source-atop";
    g.fillStyle = rgba(SLIVER, sliver);
    g.fillRect(-PAD, -PAD, STRIP.w + 2 * PAD, STRIP.h + 2 * PAD);
  }
  return face;
}

/** The light: above the card and a little left of it, in the strip's x–z plane. */
const LIGHT = { x: -0.34, z: 0.94 };

/** How dark a column's face is, 0 lit as it lies flat, by its angle; and whether its back shows. */
function shading(th: number): { dark: number; back: boolean } {
  const c = Math.cos(th);
  const s = Math.sin(th);
  const back = c < 0;
  // The lit side's normal: (sin θ, cos θ) for the face, the reverse for the back.
  const n = back ? { x: -s, z: -c } : { x: s, z: c };
  const lit = n.x * LIGHT.x + n.z * LIGHT.z;
  return { dark: clamp((LIGHT.z - lit) * 0.55, 0, 0.62), back };
}

/** Screen geometry of sample j under a pose (before the pose's transform). */
function screenOf(q: Sample, yc: number): { x: number; yc: number; m: number } {
  return { x: q.x, yc: yc - LIFT * q.z, m: 1 + q.z / PERSP };
}

/** Apply the pose's in-plane transform about its pivot. */
function poseTransform(ctx: CanvasRenderingContext2D, pose: StripPose, pivotY = pose.pivot.y): void {
  if (pose.tilt === 0 && pose.scale === 1 && pose.squash === 1) return;
  ctx.translate(pose.pivot.x, pivotY);
  ctx.rotate(pose.tilt);
  ctx.scale(pose.scale, pose.scale * pose.squash);
  ctx.translate(-pose.pivot.x, -pivotY);
}

/** A column's quad: its left and right edges' x, top and bottom, and how dark it is. */
interface Quad {
  ax: number;
  aTop: number;
  aBot: number;
  bx: number;
  bTop: number;
  bBot: number;
  alpha: number;
}

/** The pose's in-plane transform about (px, py) as a matrix. */
function poseMatrix(pose: StripPose, px: number, py: number): DOMMatrix {
  const m = new DOMMatrix();
  if (pose.tilt === 0 && pose.scale === 1 && pose.squash === 1) return m;
  return m
    .translate(px, py)
    .rotate((pose.tilt * 180) / Math.PI)
    .scale(pose.scale, pose.scale * pose.squash)
    .translate(-px, -py);
}

/**
 * Dark quads under `local` (then the frame's transform), drawn on a layer
 * and blurred as one, so a shadow of many columns has no seams and softens
 * as a whole.
 */
function drawSoftQuads(ctx: CanvasRenderingContext2D, local: DOMMatrix, quads: readonly Quad[], blur: number, alpha: number): void {
  if (!quads.length || alpha <= 0) return;
  const m = ctx.getTransform().multiply(local);
  const d = deviceScale(ctx);
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const q of quads) {
    for (const [x, y] of [
      [q.ax, q.aTop],
      [q.ax, q.aBot],
      [q.bx, q.bTop],
      [q.bx, q.bBot],
    ]) {
      const dx = m.a * x + m.c * y + m.e;
      const dy = m.b * x + m.d * y + m.f;
      x0 = Math.min(x0, dx);
      y0 = Math.min(y0, dy);
      x1 = Math.max(x1, dx);
      y1 = Math.max(y1, dy);
    }
  }
  const margin = Math.ceil(blur * d * 3 + 4);
  x0 = Math.floor(x0) - margin;
  y0 = Math.floor(y0) - margin;
  const w = Math.ceil(x1) + margin - x0;
  const h = Math.ceil(y1) + margin - y0;
  if (!(w > 0 && h > 0)) return;
  if (!shade || shade.width < w || shade.height < h) shade = makeCanvas(Math.max(w, shade?.width ?? 0), Math.max(h, shade?.height ?? 0));
  const g = shade.getContext("2d")!;
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.globalAlpha = 1;
  g.globalCompositeOperation = "source-over";
  g.clearRect(0, 0, shade.width, shade.height);
  g.setTransform(m.a, m.b, m.c, m.d, m.e - x0, m.f - y0);
  for (const q of quads) {
    if (q.alpha <= 0) continue;
    // A hair wider than the column, so neighbours meet without a seam.
    const over = q.bx >= q.ax ? 0.6 : -0.6;
    g.fillStyle = `rgba(0,0,0,${clamp(q.alpha).toFixed(4)})`;
    g.beginPath();
    g.moveTo(q.ax, q.aTop);
    g.lineTo(q.bx + over, q.bTop);
    g.lineTo(q.bx + over, q.bBot);
    g.lineTo(q.ax, q.aBot);
    g.closePath();
    g.fill();
  }
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha *= alpha;
  ctx.filter = `blur(${(blur * d).toFixed(2)}px)`;
  ctx.drawImage(shade, 0, 0, w, h, x0, y0, w, h);
  ctx.restore();
}

/**
 * The strip's two shadows. On the card: each lifted column's footprint,
 * where the light from above puts it, darker and sharper the nearer it is.
 * And right under the strip on screen, the lift a flat interface shows: a
 * short soft drop, so the strip reads as off the card wherever it is.
 */
function drawShadows(ctx: CanvasRenderingContext2D, pose: StripPose): void {
  const q = pose.samples;
  const h = STRIP.h;
  const ground: Quad[] = [];
  const drop: Quad[] = [];
  let zSum = 0;
  for (let j = 0; j < q.length - 1; j++) {
    const a = q[j];
    const b = q[j + 1];
    const z = (a.z + b.z) / 2;
    if (z <= 0.5) continue;
    zSum += z;
    ground.push({
      ax: a.x + SHADOW_DX * a.z,
      aTop: pose.yc + SHADOW_DY * a.z - h / 2,
      aBot: pose.yc + SHADOW_DY * a.z + h / 2,
      bx: b.x + SHADOW_DX * b.z,
      bTop: pose.yc + SHADOW_DY * b.z - h / 2,
      bBot: pose.yc + SHADOW_DY * b.z + h / 2,
      alpha: 0.6 * smoothstep(0, 10, z) * Math.exp(-z / 280),
    });
    const sa = screenOf(a, pose.yc);
    const sb = screenOf(b, pose.yc);
    const lift = 5 + 0.035 * z;
    drop.push({
      ax: sa.x,
      aTop: sa.yc - (sa.m * h) / 2 + lift,
      aBot: sa.yc + (sa.m * h) / 2 + lift,
      bx: sb.x,
      bTop: sb.yc - (sb.m * h) / 2 + lift,
      bBot: sb.yc + (sb.m * h) / 2 + lift,
      alpha: 0.5 * smoothstep(0, 14, z),
    });
  }
  if (!ground.length) return;
  const zMean = zSum / ground.length;
  const a = pose.alpha;
  drawSoftQuads(ctx, poseMatrix(pose, pose.pivot.x, pose.yc), ground, 1.5 + 0.045 * zMean, pose.shadow * a);
  drawSoftQuads(ctx, poseMatrix(pose, pose.pivot.x, pose.pivot.y), drop, 4 + 0.02 * zMean, a * (1 - pose.sliver));
}

/**
 * The strip in a pose: its shadow, then each column as a sheared slice of
 * its face (or its back), shaded by the light, with a highlight on the lip
 * where it bends at the peel front.
 */
export function drawStrip(ctx: CanvasRenderingContext2D, pose: StripPose): void {
  if (pose.alpha <= 0) return;
  drawShadows(ctx, pose);
  const d = deviceScale(ctx);
  const tex = faceTexture(d, pose.sliver);
  const q = pose.samples;
  const h = STRIP.h;
  ctx.save();
  ctx.globalAlpha *= pose.alpha;
  poseTransform(ctx, pose);
  // The flat run from the first flat column to the far end as one slice, so
  // a strip lying still is drawn exactly as the card draws it.
  let firstFlat = q.length - 1;
  while (firstFlat > 0 && q[firstFlat - 1].z === 0 && q[firstFlat - 1].th === 0 && q[firstFlat].z === 0) firstFlat--;
  if (firstFlat < q.length - 1) {
    const s0 = firstFlat * COL;
    const x = q[firstFlat].x;
    ctx.drawImage(tex, (PAD + s0) * d, 0, (STRIP.w - s0 + PAD) * d, (STRIP.h + 2 * PAD) * d, x, pose.yc - h / 2 - PAD, STRIP.w - s0 + PAD, h + 2 * PAD);
  }
  // The bent run, one sheared column at a time, from the far end back to the
  // free end so the curl draws over the strip it rises from.
  const OVER = 0.8;
  for (let j = firstFlat - 1; j >= 0; j--) {
    const a = screenOf(q[j], pose.yc);
    const b = screenOf(q[j + 1], pose.yc);
    const hm = ((a.m + b.m) / 2) * h;
    const ta = a.yc - (a.m * h) / 2;
    const tb = b.yc - (b.m * h) / 2;
    const th = (q[j].th + q[j + 1].th) / 2;
    const { dark, back } = shading(th);
    ctx.save();
    // Column-local u along the strip (0..COL), v down it (0..h).
    ctx.transform((b.x - a.x) / COL, (tb - ta) / COL, 0, hm / h, a.x, ta);
    const ext = j === 0 ? 0 : OVER;
    if (back) {
      ctx.fillStyle = STRIP_BACK;
      ctx.fillRect(-ext, 0, COL + 2 * ext, h);
    } else {
      const sx = (PAD + j * COL - ext) * d;
      ctx.drawImage(tex, sx, 0, (COL + 2 * ext) * d, (STRIP.h + 2 * PAD) * d, -ext, -PAD, COL + 2 * ext, h + 2 * PAD);
    }
    if (dark > 0.005) {
      ctx.fillStyle = `rgba(4,10,16,${dark.toFixed(4)})`;
      ctx.fillRect(-ext, 0, COL + 2 * ext, h);
    }
    ctx.restore();
  }
  // The lip: light catching the bend at the peel front.
  const lip = q.findIndex((p, j) => j < q.length - 1 && p.th > 0.12 && q[j + 1].th <= 0.12);
  if (lip > 0 && pose.scale === 1) {
    const p = screenOf(q[lip], pose.yc);
    const g = ctx.createLinearGradient(p.x - 10, 0, p.x + 4, 0);
    g.addColorStop(0, rgba(PALETTE.paper, 0));
    g.addColorStop(0.65, rgba(PALETTE.paper, 0.22));
    g.addColorStop(1, rgba(PALETTE.paper, 0));
    ctx.fillStyle = g;
    ctx.fillRect(p.x - 10, p.yc - (p.m * h) / 2, 14, p.m * h);
  }
  ctx.restore();
}

// The blade and what it throws.

/** A four-point star, r px, for the blade's glint before it moves. */
function star(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, alpha: number): void {
  if (r <= 0 || alpha <= 0) return;
  glow(ctx, x, y, r * 2.2, PALETTE.glint, 0.55 * alpha);
  const c = r * 0.13;
  ctx.save();
  ctx.globalAlpha *= alpha;
  ctx.fillStyle = PALETTE.glint;
  ctx.beginPath();
  ctx.moveTo(x, y - r * 0.7);
  ctx.quadraticCurveTo(x + c, y - c, x + r, y);
  ctx.quadraticCurveTo(x + c, y + c, x, y + r * 0.7);
  ctx.quadraticCurveTo(x - c, y + c, x - r, y);
  ctx.quadraticCurveTo(x - c, y - c, x, y - r * 0.7);
  ctx.fill();
  ctx.restore();
}

/**
 * The slice: a glint at the card's left edge on the upbeat before (the
 * anticipation), then a #ecfdff blade that sweeps the cut line in a
 * sixteenth and leaves it glowing, fading behind it.
 */
export function drawBlade(ctx: CanvasRenderingContext2D, b: number): void {
  const y = CUT.y;
  // The glint gathers on the card's edge, where the blade will enter.
  const pre = progress(1.8125, 2, b);
  if (pre > 0 && b < 2.1875) {
    const fade = 1 - progress(2, 2.1875, b);
    star(ctx, CARD_X0 - 6, y, 26 * Math.sin((Math.PI / 2) * pre) * fade, fade);
  }
  if (b < CUT.from || b > CUT.to + 0.5) return;
  const head = bladeX(b);
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  // The cut behind it: bright where the blade has just been, fading over a
  // few hundredths of a second, only across the card.
  const cutTo = Math.min(head, CARD_X1);
  if (cutTo > CARD_X0) {
    const g = ctx.createLinearGradient(CARD_X0, 0, CARD_X1, 0);
    for (let i = 0; i <= 12; i++) {
      const x = lerp(CARD_X0, CARD_X1, i / 12);
      const since = x <= head ? (b - bladeAt(x)) * BEAT : -1;
      const a = since < 0 ? 0 : 0.9 * Math.exp(-since / 0.06);
      g.addColorStop(i / 12, rgba(PALETTE.glint, a));
    }
    ctx.fillStyle = g;
    ctx.fillRect(CARD_X0, y - 1.25, cutTo - CARD_X0, 2.5);
  }
  if (b <= CUT.to + 0.0625) {
    // The blade: a white-hot head with a tail as long as its speed.
    const fade = 1 - progress(CUT.to, CUT.to + 0.0625, b);
    const speed = (bladeX(b + 0.01) - bladeX(b - 0.01)) / 0.02 / 2728; // 0..~1.6 of the mean
    const len = 60 + 200 * clamp(speed, 0, 1.5);
    const g = ctx.createLinearGradient(head - len, 0, head, 0);
    g.addColorStop(0, rgba(PALETTE.glint, 0));
    g.addColorStop(0.75, rgba(PALETTE.glint, 0.55 * fade));
    g.addColorStop(1, rgba(PALETTE.glint, fade));
    ctx.fillStyle = g;
    ctx.beginPath();
    ctx.moveTo(head - len, y);
    ctx.lineTo(head - 10, y - 2.5);
    ctx.lineTo(head + 6, y);
    ctx.lineTo(head - 10, y + 2.5);
    ctx.closePath();
    ctx.fill();
    glow(ctx, head, y, 44, PALETTE.glint, 0.75 * fade);
    // A thin flare across the edge at the head.
    const f = ctx.createLinearGradient(0, y - 18, 0, y + 18);
    f.addColorStop(0, rgba(PALETTE.glint, 0));
    f.addColorStop(0.5, rgba(PALETTE.glint, 0.8 * fade));
    f.addColorStop(1, rgba(PALETTE.glint, 0));
    ctx.fillStyle = f;
    ctx.fillRect(head - 0.75, y - 18, 1.5, 36);
  }
  ctx.restore();
}

/** Sparks thrown off the card's right edge as the blade leaves it: seeded, ballistic, cooling from white to warm. */
const SPARKS = Array.from({ length: 18 }, (_, i) => ({
  at: SPARKS_AT + (i % 6) * 0.012,
  angle: (-58 + 66 * hash(i, 71)) * (Math.PI / 180),
  speed: 700 + 950 * hash(i, 72),
  life: 0.06 + 0.1 * hash(i, 73),
  width: 1.4 + 1.4 * hash(i, 74),
}));
const GRAVITY = 2600;
/** No spark reaches the captions' band. */
const SPARK_FLOOR = 728;

export function drawSparks(ctx: CanvasRenderingContext2D, b: number): void {
  if (b < SPARKS_AT || b > SPARKS_AT + 1) return;
  const x0 = CARD_X1;
  const y0 = CUT.y;
  const pos = (s: (typeof SPARKS)[number], age: number) => ({
    x: x0 + s.speed * Math.cos(s.angle) * age,
    y: y0 + s.speed * Math.sin(s.angle) * age + 0.5 * GRAVITY * age * age,
  });
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  ctx.lineCap = "round";
  // The flash where it leaves.
  const since = (b - SPARKS_AT) * BEAT;
  glow(ctx, x0, y0, 60, PALETTE.glint, 0.9 * Math.exp(-since / 0.04));
  for (const s of SPARKS) {
    const age = (b - s.at) * BEAT;
    if (age <= 0 || age >= s.life) continue;
    const k = age / s.life;
    const head = pos(s, age);
    const tail = pos(s, Math.max(0, age - 0.022));
    const a = (1 - k) ** 0.8 * (1 - smoothstep(SPARK_FLOOR - 40, SPARK_FLOOR, head.y));
    if (a <= 0) continue;
    glow(ctx, head.x, head.y, 9 * (1 - 0.5 * k), mix(PALETTE.glint, PALETTE.warm, k), 0.5 * a);
    ctx.strokeStyle = mix(PALETTE.glint, PALETTE.warm, k, a);
    ctx.lineWidth = s.width * (1 - 0.6 * k);
    ctx.beginPath();
    ctx.moveTo(tail.x, tail.y);
    ctx.lineTo(head.x, head.y);
    ctx.stroke();
  }
  ctx.restore();
}
