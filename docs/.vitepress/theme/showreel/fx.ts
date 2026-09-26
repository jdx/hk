// Shared finishing effects. Offscreen canvases are created lazily on first
// use so importing this module is safe during server-side rendering.

import type { LitRect } from "./bible";
import { rgba } from "./color";
import { clamp, hash } from "./math";

/** PALETTE.cyanBright: flashes and rings default to hk's bright brand cyan. */
const BRIGHT = "#8de1e8";

export function makeCanvas(w: number, h: number): HTMLCanvasElement {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  return c;
}

const glowCache = new Map<string, HTMLCanvasElement>();

/** Additive soft light. Cheap: one cached sprite per color. */
export function glow(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  radius: number,
  color: string,
  alpha = 1,
): void {
  if (alpha <= 0 || radius <= 0) return;
  let sprite = glowCache.get(color);
  if (!sprite) {
    sprite = makeCanvas(128, 128);
    const g = sprite.getContext("2d")!;
    const grad = g.createRadialGradient(64, 64, 0, 64, 64, 64);
    grad.addColorStop(0, rgba(color, 1));
    grad.addColorStop(0.2, rgba(color, 0.55));
    grad.addColorStop(0.5, rgba(color, 0.16));
    grad.addColorStop(1, rgba(color, 0));
    g.fillStyle = grad;
    g.fillRect(0, 0, 128, 128);
    glowCache.set(color, sprite);
  }
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  ctx.globalAlpha *= clamp(alpha);
  ctx.drawImage(sprite, x - radius, y - radius, radius * 2, radius * 2);
  ctx.restore();
}

let grainTiles: HTMLCanvasElement[] | null = null;

/** Film grain, re-seeded 24 times a second so it reads as film, not noise. */
export function grain(
  ctx: CanvasRenderingContext2D,
  W: number,
  H: number,
  t: number,
  amount = 0.06,
): void {
  if (amount <= 0) return;
  if (!grainTiles) {
    grainTiles = [];
    for (let k = 0; k < 4; k++) {
      const c = makeCanvas(256, 256);
      const g = c.getContext("2d")!;
      const img = g.createImageData(256, 256);
      for (let i = 0; i < 256 * 256; i++) {
        const v = hash(i, k + 11) * 255;
        img.data[i * 4] = v;
        img.data[i * 4 + 1] = v;
        img.data[i * 4 + 2] = v;
        img.data[i * 4 + 3] = 255;
      }
      g.putImageData(img, 0, 0);
      grainTiles.push(c);
    }
  }
  const f = Math.floor(t * 24);
  const tile = grainTiles[f % 4];
  const pattern = ctx.createPattern(tile, "repeat");
  if (!pattern) return;
  ctx.save();
  ctx.globalCompositeOperation = "overlay";
  ctx.globalAlpha = amount;
  ctx.translate(-hash(f, 3) * 256, -hash(f, 5) * 256);
  ctx.fillStyle = pattern;
  ctx.fillRect(0, 0, W + 256, H + 256);
  ctx.restore();
}

let vignetteLayer: HTMLCanvasElement | null = null;

/**
 * How far round a lit screen the vignette is spared, in the screen's half
 * widths and heights: all of it inside the ellipse through its corners,
 * fading back to none twice as far out.
 */
const SPARE_CORE = Math.SQRT2;
const SPARE_OUT = 2 * Math.SQRT2;

/**
 * Darkens the frame toward its edges. A lit screen, if given, is left out
 * of it: the ellipse through its corners spared by `lit.alpha`, fading back
 * into the vignette with no edge. Without one the vignette is painted
 * straight onto the frame.
 */
export function vignette(
  ctx: CanvasRenderingContext2D,
  W: number,
  H: number,
  strength = 0.55,
  lit: LitRect | null = null,
): void {
  const paint = (c: CanvasRenderingContext2D) => {
    const g = c.createRadialGradient(W / 2, H / 2, H * 0.35, W / 2, H / 2, H * 1.05);
    g.addColorStop(0, "rgba(0,0,0,0)");
    g.addColorStop(1, `rgba(0,0,0,${strength})`);
    c.fillStyle = g;
    c.fillRect(0, 0, W, H);
  };
  const spare = lit ? clamp(lit.alpha) : 0;
  if (!lit || spare <= 0 || lit.w < 1 || lit.h < 1) {
    ctx.save();
    paint(ctx);
    ctx.restore();
    return;
  }
  // On a layer of its own, so the spared ellipse can be cut out of it.
  const { width, height } = ctx.canvas;
  vignetteLayer ??= makeCanvas(width, height);
  if (vignetteLayer.width !== width || vignetteLayer.height !== height) {
    vignetteLayer.width = width;
    vignetteLayer.height = height;
  }
  const v = vignetteLayer.getContext("2d")!;
  v.setTransform(1, 0, 0, 1, 0, 0);
  v.globalAlpha = 1;
  v.globalCompositeOperation = "source-over";
  v.clearRect(0, 0, width, height);
  v.setTransform(ctx.getTransform());
  paint(v);
  v.globalCompositeOperation = "destination-out";
  v.translate(lit.x + lit.w / 2, lit.y + lit.h / 2);
  v.scale(lit.w / 2, lit.h / 2);
  const s = v.createRadialGradient(0, 0, 0, 0, 0, SPARE_OUT);
  const core = SPARE_CORE / SPARE_OUT;
  s.addColorStop(0, `rgba(0,0,0,${spare})`);
  for (let i = 0; i <= 8; i++) {
    const u = i / 8;
    s.addColorStop(core + (1 - core) * u, `rgba(0,0,0,${spare * (1 - u * u * (3 - 2 * u))})`);
  }
  v.fillStyle = s;
  v.fillRect(-SPARE_OUT, -SPARE_OUT, 2 * SPARE_OUT, 2 * SPARE_OUT);
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha = 1;
  ctx.globalCompositeOperation = "source-over";
  ctx.drawImage(vignetteLayer, 0, 0);
  ctx.restore();
}

/** Full-frame flash, e.g. one or two frames on a hard cut. */
export function flash(
  ctx: CanvasRenderingContext2D,
  W: number,
  H: number,
  alpha: number,
  color = BRIGHT,
): void {
  if (alpha <= 0) return;
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  ctx.fillStyle = rgba(color, clamp(alpha));
  ctx.fillRect(0, 0, W, H);
  ctx.restore();
}

/** Expanding shockwave ring. `p` runs 0..1 over its life. */
export function ring(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  radius: number,
  p: number,
  color = BRIGHT,
  width = 6,
): void {
  if (p <= 0 || p >= 1) return;
  const e = 1 - (1 - p) ** 3;
  const r = radius * e;
  ctx.save();
  ctx.strokeStyle = rgba(color, (1 - p) ** 1.5);
  // Never thicker than the ring is wide, so it does not start as a dot.
  ctx.lineWidth = Math.min(width * (1 - p) + 0.5, r);
  ctx.beginPath();
  ctx.arc(x, y, r, 0, Math.PI * 2);
  ctx.stroke();
  ctx.restore();
}

/**
 * Horizontal motion smear: draws `draw` several times along the motion with
 * falling opacity. `offset` is where the content sits, `velocity` how far it
 * travels in one frame (px); the trail extends behind it.
 */
export function smear(
  ctx: CanvasRenderingContext2D,
  offset: number,
  velocity: number,
  draw: () => void,
  samples = 6,
): void {
  const n = Math.abs(velocity) < 2 ? 1 : samples;
  for (let i = n - 1; i >= 0; i--) {
    ctx.save();
    ctx.translate(offset - (velocity * i) / n, 0);
    ctx.globalAlpha *= i === 0 ? 1 : 0.35 * (1 - i / n);
    draw();
    ctx.restore();
  }
}

/** Rounded rectangle path (independent of CanvasRenderingContext2D.roundRect). */
export function roundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  r: number,
): void {
  const rr = Math.max(0, Math.min(r, w / 2, h / 2));
  ctx.beginPath();
  ctx.moveTo(x + rr, y);
  ctx.arcTo(x + w, y, x + w, y + h, rr);
  ctx.arcTo(x + w, y + h, x, y + h, rr);
  ctx.arcTo(x, y + h, x, y, rr);
  ctx.arcTo(x, y, x + w, y, rr);
  ctx.closePath();
}

/**
 * Deterministic screen shake offset for an impact at `at`. Zero on the
 * impact instant itself, so a shake keyed to a cut keeps the handoff exact;
 * it lasts about five times `decay`.
 */
export function shake(t: number, at: number, amp = 10, decay = 0.18): [number, number] {
  const d = t - at;
  if (d <= 0 || d > decay * 5) return [0, 0];
  const k = amp * Math.exp(-d / decay);
  const f = Math.floor(d * 60);
  return [(hash(f, 17) * 2 - 1) * k, (hash(f, 29) * 2 - 1) * k];
}
