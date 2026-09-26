// The catch scene's props (storyboard §6.7): the commit card with its loop
// and ✗ badge, the fishing line, the ✗ glyph at any size, and the soft
// shadows and halos that lift them off the stage. Pure drawing: every
// function takes where and how far, and the scene (catch.ts) owns the clock.
//
// The hook itself is the kit's (logo.ts drawIconHook). The geometry here
// hangs the card's loop in the hook's bowl: a loaded hook turns about its
// eye until the bowl's centre is under the eye, so the loop rests at the
// bowl's lowest point, straight under the line.

import { PALETTE } from "../bible";
import { mix, rgba } from "../color";
import { glow, roundedRect } from "../fx";
import { drawMono, drawPanel } from "../kit/card";
import { ICON_HOOK, LOGO_CENTER, type LogoPlace, logoScale } from "../kit/logo";
import type { Pt } from "../kit/motion";
import { finding } from "../kit/screens";
import { clamp, DEG, hash, TAU } from "../math";

// The hook and the loop.

/** px per logo unit: storyboard §6.7 says 3; 3.5 holds up better on a thumbnail. */
export const HOOK_SCALE = 3.5;
/** The hook's stroke, px. */
export const HOOK_W = ICON_HOOK.stroke * HOOK_SCALE;
/** From the eye to the bowl's centre, px. */
export const EYE_BOWL = Math.hypot(ICON_HOOK.bowl.cx - ICON_HOOK.eye.cx, ICON_HOOK.bowl.cy - ICON_HOOK.eye.cy) * HOOK_SCALE;
/** The bowl's radius on the stroke's centre line, px. */
export const BOWL_R = ICON_HOOK.bowl.r * HOOK_SCALE;
/**
 * The loaded hook's turn about its eye, degrees (negative is anticlockwise):
 * it puts the bowl's centre, and so its lowest point, straight under the eye.
 */
export const HOOK_TILT = -Math.atan2(ICON_HOOK.eye.cx - ICON_HOOK.bowl.cx, ICON_HOOK.bowl.cy - ICON_HOOK.eye.cy) / DEG;
/** Where the line is tied: the top of the eye ring's ink, px above the eye. */
export const TIE = (ICON_HOOK.eye.r + ICON_HOOK.ring / 2) * HOOK_SCALE;

/**
 * The card's loop: a key ring through a hole punched under the card's top
 * edge, wide enough for the hook's stroke to pass through it (the
 * storyboard's r 10 would not take a hook this size), and long enough that
 * the loaded card hangs clear of the bowl: a strip of stage about 15 px
 * tall shows between the bowl and the card's top edge, crossed by the
 * ring's two sides, so the card reads as hung by its loop.
 */
export const LOOP_R = 26;
export const LOOP_W = 5;
/** The punched hole's centre below the card's top edge, and its radius. */
export const HOLE_Y = 12;
const HOLE_R = 7;
/** The loop's centre relative to the card's top edge: its lowest point is in the hole. */
export const loopCentre = (cx: number, top: number): Pt => ({ x: cx, y: top + HOLE_Y - LOOP_R });
/**
 * The loop's centre below the eye when it hangs in the bowl: its inner top
 * edge rests on the stroke at the bowl's lowest point.
 */
export const LOOP_BELOW_EYE = EYE_BOWL + BOWL_R - HOOK_W / 2 + LOOP_R - LOOP_W / 2;
/** The card's top edge below the eye when it hangs. */
export const CARD_BELOW_EYE = LOOP_BELOW_EYE - HOLE_Y + LOOP_R;

/**
 * Where the loop's centre line crosses the bowl's, as an angle on the loop
 * (radians, canvas convention, 0 to the right): the loop passes in front of
 * the hook on the right of the bowl and behind it on the left, so the two
 * read as linked.
 */
export const LOOP_CROSS = (() => {
  const d = LOOP_BELOW_EYE - EYE_BOWL;
  const a = Math.acos((LOOP_R * LOOP_R + d * d - BOWL_R * BOWL_R) / (2 * LOOP_R * d));
  return -Math.PI / 2 + a;
})();

/**
 * The hole and the loop through it, on a card whose top centre is (cx, top):
 * the ring runs into the hole, so nothing of it shows below the hole's
 * centre. `sway` turns it about the hole, degrees clockwise.
 */
export function drawLoop(ctx: CanvasRenderingContext2D, cx: number, top: number, sway = 0): void {
  const hy = top + HOLE_Y;
  ctx.save();
  ctx.fillStyle = PALETTE.bg;
  ctx.strokeStyle = PALETTE.divider;
  ctx.lineWidth = 1.5;
  ctx.beginPath();
  ctx.arc(cx, hy, HOLE_R, 0, TAU);
  ctx.fill();
  ctx.stroke();
  ctx.translate(cx, hy);
  if (sway) ctx.rotate(sway * DEG);
  ctx.beginPath();
  ctx.rect(-LOOP_R - LOOP_W, -2 * LOOP_R - LOOP_W, 2 * LOOP_R + 2 * LOOP_W, 2 * LOOP_R + LOOP_W);
  ctx.clip();
  ctx.strokeStyle = PALETTE.text2;
  ctx.lineWidth = LOOP_W;
  ctx.beginPath();
  ctx.arc(0, -LOOP_R, LOOP_R, 0, TAU);
  ctx.stroke();
  ctx.restore();
}

/** The part of the loop that passes in front of the hook's stroke. */
export function drawLoopFront(ctx: CanvasRenderingContext2D, c: Pt, alpha = 1): void {
  if (alpha <= 0) return;
  ctx.save();
  ctx.globalAlpha *= alpha;
  ctx.strokeStyle = PALETTE.text2;
  ctx.lineWidth = LOOP_W;
  ctx.lineCap = "butt";
  ctx.beginPath();
  ctx.arc(c.x, c.y, LOOP_R, LOOP_CROSS - 0.62, LOOP_CROSS + 0.62);
  ctx.stroke();
  ctx.restore();
}

/** The hook's shank, bowl and point as one run, logo units, and its length. */
const HOOK_RUN = "M132 74 V92 A13 13 0 0 1 106 92 V84";
const HOOK_RUN_LEN = 18 + 13 * Math.PI + 8;
let hookRun: Path2D | null = null;

/**
 * A highlight running down the icon hook as `u` goes 0 to 1, from under the
 * eye round the bowl to the point: the logo's glint (logo.ts glint), on the
 * hook drawn at `place` turned `turn` degrees about its eye.
 */
export function hookGlint(ctx: CanvasRenderingContext2D, place: LogoPlace, turn: number, u: number): void {
  if (!(u > 0 && u < 1)) return;
  hookRun ??= new Path2D(HOOK_RUN);
  const s = logoScale(place);
  const { cx: ex, cy: ey } = ICON_HOOK.eye;
  ctx.save();
  ctx.transform(s, 0, 0, s, place.cx - LOGO_CENTER[0] * s, place.cy - LOGO_CENTER[1] * s);
  ctx.translate(ex, ey);
  ctx.rotate(turn * DEG);
  ctx.translate(-ex, -ey);
  // In and out softly, so the dash never appears or vanishes whole.
  ctx.strokeStyle = rgba(PALETTE.glint, 0.9 * Math.sin(Math.PI * u) ** 0.5);
  ctx.lineWidth = 2.2;
  ctx.lineCap = "round";
  ctx.setLineDash([9, 400]);
  ctx.lineDashOffset = 9 - u * (HOOK_RUN_LEN + 9);
  ctx.stroke(hookRun);
  ctx.restore();
}

// The line.

export interface LineWave {
  /** A travelling ripple, px at its widest, and its phase in waves. */
  ripple?: number;
  phase?: number;
  /** A standing vibration of the taut line, px at its middle. */
  twang?: number;
}

/**
 * The fishing line from `a` to `b` (the eye ring's top): 3 px logo cyan at
 * 0.75, a little heavier than the kit's 2 px so it survives a thumbnail. A
 * ripple travels down it as it is cast; a taut line twangs.
 */
export function drawFishingLine(ctx: CanvasRenderingContext2D, a: Pt, b: Pt, w: LineWave = {}, alpha = 1): void {
  if (alpha <= 0 || b.y <= a.y) return;
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy);
  const nx = -dy / len;
  const ny = dx / len;
  const n = 48;
  ctx.save();
  ctx.globalAlpha *= alpha;
  ctx.strokeStyle = rgba(PALETTE.logo, 0.75);
  ctx.lineWidth = 3;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  for (let i = 0; i <= n; i++) {
    const s = i / n;
    const env = Math.sin(Math.PI * s);
    const off = (w.ripple ?? 0) * env * env * Math.sin(TAU * (2.5 * s - (w.phase ?? 0))) + (w.twang ?? 0) * env;
    const x = a.x + dx * s + nx * off;
    const y = a.y + dy * s + ny * off;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  }
  ctx.stroke();
  ctx.restore();
}

// Glyphs.

/**
 * hk's ✗ (term.ts draws it in a terminal cell) centred on (x, y), `h` px
 * tall: the falling stroke straight, the rising one bowed, as by hand.
 */
export function drawCross(ctx: CanvasRenderingContext2D, x: number, y: number, h: number, color: string, weight = 0.11): void {
  if (h <= 0) return;
  // The glyph's ink in em units: x 0.072–0.516, y −0.64 to −0.04.
  const u = h / 0.6;
  const w = 0.6 * u;
  const ox = x - 0.49 * w;
  const oy = y + 0.34 * u;
  ctx.save();
  ctx.strokeStyle = color;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.lineWidth = weight * u;
  ctx.beginPath();
  ctx.moveTo(ox + 0.14 * w, oy - 0.62 * u);
  ctx.lineTo(ox + 0.86 * w, oy - 0.04 * u);
  ctx.moveTo(ox + 0.86 * w, oy - 0.64 * u);
  ctx.quadraticCurveTo(ox + 0.42 * w, oy - 0.3 * u, ox + 0.12 * w, oy - 0.04 * u);
  ctx.stroke();
  ctx.restore();
}

// Depth.

/** A soft drop shadow under a rounded rect: stacked, widening, fainter rects. */
export function softShadow(ctx: CanvasRenderingContext2D, r: { x: number; y: number; w: number; h: number }, o: { dy?: number; spread?: number; alpha?: number; radius?: number } = {}): void {
  const alpha = o.alpha ?? 0.5;
  if (alpha <= 0) return;
  const spread = o.spread ?? 28;
  const dy = o.dy ?? 14;
  const n = 7;
  ctx.save();
  ctx.fillStyle = "#000";
  for (let i = 0; i < n; i++) {
    const g = (spread * (i + 1)) / n;
    ctx.globalAlpha = (alpha / n) * (1 - i / (n + 1)) * 1.6;
    roundedRect(ctx, r.x - g / 2, r.y + dy - g / 2, r.w + g, r.h + g, (o.radius ?? 16) + g / 2);
    ctx.fill();
  }
  ctx.restore();
}

/** Light round a rounded rect's edge, added to what is under it. */
export function halo(ctx: CanvasRenderingContext2D, r: { x: number; y: number; w: number; h: number }, color: string, alpha: number, spread = 26, radius = 16): void {
  if (alpha <= 0) return;
  // Enough thin rings, each overlapping the next, that no step shows.
  const n = 16;
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  for (let i = 0; i < n; i++) {
    const g = (spread * (i + 0.5)) / n;
    ctx.strokeStyle = rgba(color, (alpha / n) * (1 - i / n) ** 2 * 3);
    ctx.lineWidth = (2 * spread) / n;
    roundedRect(ctx, r.x - g, r.y - g, r.w + 2 * g, r.h + 2 * g, radius + g);
    ctx.stroke();
  }
  ctx.restore();
}

/**
 * A little dust off the rail where a corner lands: `n` motes drifting out
 * and up from (x, y), `dir` −1 left or 1 right, over `u` 0..1.
 */
export function puff(ctx: CanvasRenderingContext2D, x: number, y: number, dir: number, u: number, seed: number, n = 5, reach = 36): void {
  if (!(u > 0 && u < 1)) return;
  ctx.save();
  for (let i = 0; i < n; i++) {
    const h1 = hash(i, seed);
    const h2 = hash(i, seed + 7);
    const k = 1 - (1 - u) ** 2;
    const px = x + dir * (8 + reach * (0.4 + 0.6 * h1) * k);
    const py = y - (6 + 18 * h2) * k + 10 * u * u;
    ctx.fillStyle = rgba(PALETTE.text3, 0.55 * (1 - u) * (0.6 + 0.4 * h2));
    ctx.beginPath();
    ctx.arc(px, py, 2 + 2.5 * h1 * (1 - 0.5 * u), 0, TAU);
    ctx.fill();
  }
  ctx.restore();
}

// The commit card.

/** The card riding in, and grown on the ✗ (storyboard §6.7; widened to hold its lines at 36 px). */
export const CARD_SMALL = { w: 440, h: 144 } as const;
export const CARD_BIG = { w: 560, h: 190 } as const;
/** Mono size of every line on the card, so the carets sit under `unused` as shellcheck prints them. */
export const CARD_TEXT = 36;
const PAD_X = 30;
const ROW0 = 60;
const ROW_H = 48;

export interface CardLook {
  /** How red the edge is, 0..1 (the ✗). */
  red: number;
  /** A flash of red over the whole card, 0..1. */
  flash: number;
  /** The warning line wiped on from the left, 0..1. */
  warn: number;
  /** The ✗ badge's scale (a spring that overshoots) and its turn, degrees. */
  badge: number;
  badgeTurn: number;
  /** The badge's glow, 0..1. */
  badgeGlow: number;
  /** A drop shadow's strength, 0..1. */
  shadow: number;
  /** The loop's sway about its hole, degrees. */
  sway: number;
}

/** The badge's centre on a card whose top-right corner is (right, top). */
export const badgeAt = (right: number, top: number): Pt => ({ x: right - 14, y: top + 14 });
export const BADGE_R = 34;

/**
 * The commit card: `rect` in the caller's frame (the scene has already
 * turned and shaken it). The shadow, the panel with its edge going red, the
 * file, `unused=1`, shellcheck's caret line, the loop through its hole at
 * the top centre, and the badge.
 */
export function drawCommitCard(ctx: CanvasRenderingContext2D, rect: { x: number; y: number; w: number; h: number }, look: CardLook): void {
  const { x, y, w, h } = rect;
  softShadow(ctx, rect, { alpha: 0.55 * look.shadow, dy: 16, spread: 34 });
  const red = clamp(look.red);
  // The red edge lights the stage round it: hard on the ✗, then a low glow.
  halo(ctx, rect, PALETTE.red, 0.5 * clamp(look.flash) + 0.14 * red);
  drawPanel(ctx, rect, {
    fill: mix(PALETTE.surface, PALETTE.red, 0.16 * clamp(look.flash)),
    stroke: mix(PALETTE.divider, PALETTE.red, red),
    lineWidth: 1 + 1.5 * red,
  });
  // The lines, clipped to the card so its growth uncovers the third.
  ctx.save();
  roundedRect(ctx, x, y, w, h, 16);
  ctx.clip();
  const x0 = x + PAD_X;
  drawMono(ctx, finding.file, x0, y + ROW0, CARD_TEXT, PALETTE.text1);
  drawMono(ctx, finding.source, x0, y + ROW0 + ROW_H, CARD_TEXT, PALETTE.warm);
  const warn = clamp(look.warn);
  if (warn > 0) {
    const width = finding.warning.length * 0.6 * CARD_TEXT;
    ctx.beginPath();
    ctx.rect(x0 - 4, y + ROW0 + ROW_H, (width + 8) * warn, ROW_H + 10);
    ctx.clip();
    drawMono(ctx, finding.warning, x0, y + ROW0 + 2 * ROW_H, CARD_TEXT, PALETTE.red);
  }
  ctx.restore();
  drawLoop(ctx, x + w / 2, y, look.sway);
  drawBadge(ctx, badgeAt(x + w, y), look.badge, look.badgeTurn, look.badgeGlow);
}

/** The red ✗ badge: a red disc with the ✗ cut out of it in the stage's colour. */
export function drawBadge(ctx: CanvasRenderingContext2D, c: Pt, scale: number, turn: number, glowAmount: number): void {
  if (scale <= 0) return;
  glow(ctx, c.x, c.y, BADGE_R * 3.2, PALETTE.red, 0.45 * clamp(glowAmount) * Math.min(1, scale));
  ctx.save();
  ctx.translate(c.x, c.y);
  ctx.rotate(turn * DEG);
  ctx.scale(scale, scale);
  // A dark rim parts the badge from the card edge under it.
  ctx.fillStyle = PALETTE.bg;
  ctx.beginPath();
  ctx.arc(0, 0, BADGE_R + 4, 0, TAU);
  ctx.fill();
  ctx.fillStyle = PALETTE.red;
  ctx.beginPath();
  ctx.arc(0, 0, BADGE_R, 0, TAU);
  ctx.fill();
  drawCross(ctx, 0, 0, BADGE_R * 0.95, PALETTE.bg, 0.15);
  ctx.restore();
}
