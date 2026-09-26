// Scene 10, "Morph" (storyboard §6.10): the argument closes by turning the
// benchmark's bars into hk's mark, in four beats and one gesture down the
// stack. On the heartbeat's lub a light runs down the hk bar; hk's cyan then
// runs down each grey rival bar in turn, the way the bars grew in the race,
// and each bar lifts off as it is lit. It gathers, shortening toward the
// column of its stroke along its own row, then rises and turns into a
// stroke of the wordmark at LOGO_END (morph-pose.ts). The h's stem and the
// k's stem stand up; the k's arm slides in under the stem's foot already a
// diagonal and docks on the stem from the right, so no bar ever passes
// through another. Each swings a hair past its angle to land on its
// sixteenth, and rocks back to rest. The last bar is the pen: it rises into
// the k's junction, draws out along the leg's straight run, and on the
// heartbeat's dub carries straight on round the bowl to the point while the
// h's shoulder springs out of its stem. The stage dims from bg to night from
// the edges in, and from b3.5 the frame is morph|end, the wordmark minus its
// barb, which end clicks out on its downbeat.
//
// Every bar is drawn with handoff.ts's drawCapsule, so the first frame is
// race|morph, and a bar that has settled on its stroke is identical to the
// logo kit's stroke (checked in Chromium: no pixel differs), which draws it
// from then on. Every value is a function of local time.

import { BEAT, type LitRect, PALETTE, type Scene, type SceneEnv, sec } from "../bible";
import { rgba } from "../color";
import { glow, makeCanvas } from "../fx";
import { capsuleAt, drawCapsule, drawHandoff, HANDOFFS } from "../handoff";
import { drawLogo, LOGO_END, logoToPx, STROKE, strokeTip } from "../kit/logo";
import type { Pt } from "../kit/motion";
import { clamp, inOutSine, lerp, progress, smoothstep } from "../math";
import type { Caption } from "../type";
import { DUB, FILL, FILL_DUR, LAND, LIFT, LUB, PEN, STILL, SWAP } from "./morph-cues";
import { angleOf, BECOMES, legAt, lengthOf, mid, type Pose, poseAt, SETTLE, shoulderAt, TARGET } from "./morph-pose";

const S = sec("morph");

/** Local seconds of section beat `n`. */
const b = (n: number): number => n * BEAT;

/** The section's must-read captions (none: the picture carries it). */
export const CAPTIONS: readonly Caption[] = [];

// The cues (morph-cues.ts), which the score can import without the drawing.
export { DUB, FILL, LAND, LIFT, LUB, PEN, STILL, SWAP } from "./morph-cues";

// Drawing.

function strokeSeg(ctx: CanvasRenderingContext2D, a: Pt, z: Pt, width: number, style: string | CanvasGradient): void {
  ctx.save();
  ctx.strokeStyle = style;
  ctx.lineWidth = width;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(a.x, a.y);
  ctx.lineTo(z.x, z.y);
  ctx.stroke();
  ctx.restore();
}

/** Bar i waiting, and for a rival, hk's cyan running down it from the left over its fill. */
function drawWaiting(ctx: CanvasRenderingContext2D, i: number, lt: number): void {
  const c = capsuleAt(i);
  const pose = poseAt(i, lt);
  const f = i === 0 ? 0 : progress(FILL[i], FILL[i] + FILL_DUR, lt);
  if (f >= 1) {
    drawCapsule(ctx, pose.a, pose.b, pose.width, PALETTE.logo, 1);
    return;
  }
  drawCapsule(ctx, pose.a, pose.b, pose.width, c.color, c.alpha);
  if (f <= 0) return;
  // A soft front, starting and ending clear of the ink, so the colour comes
  // on without an edge and the bar is plain cyan when it has run the length.
  const soft = 90;
  const front = lerp(c.a.x - c.width / 2 - soft, c.b.x + c.width / 2 + soft, inOutSine(f));
  const g = ctx.createLinearGradient(front - soft, 0, front, 0);
  g.addColorStop(0, rgba(PALETTE.logo, 1));
  g.addColorStop(1, rgba(PALETTE.logo, 0));
  strokeSeg(ctx, pose.a, pose.b, pose.width, g);
}

/** The light at the front of each fill, and the one down the hk bar on the lub. */
function drawSparks(ctx: CanvasRenderingContext2D, lt: number): void {
  for (let i = 0; i < 4; i++) {
    const f = progress(FILL[i], FILL[i] + FILL_DUR, lt);
    if (f <= 0 || f >= 1) continue;
    const c = capsuleAt(i);
    const x = lerp(c.a.x - 20, c.b.x + 20, inOutSine(f));
    const k = Math.sin(Math.PI * f);
    glow(ctx, x, c.a.y, 110, PALETTE.glint, 0.45 * k);
    glow(ctx, x, c.a.y, 40, PALETTE.glint, 0.6 * k);
  }
}

/** Motion blur in the fast middle of a flight. */
const trailAt = (p: number): number => smoothstep(0.06, 0.2, p) * (1 - smoothstep(0.6, 0.8, p));
/** Seconds of travel the blur covers: a frame and a half at 60 fps. */
const BLUR_SPAN = 1.5 / 60;
/** The blur's copies: at least 8, one every BLUR_STEP px the bar's faster end travels, at most 48. */
const BLUR_MIN = 8;
const BLUR_MAX = 48;
const BLUR_STEP = 2.5;
/** Each copy's weight on the smear's layer at full trail, times BLUR_MIN / n; and the layer's alpha on the frame. */
const BLUR_COPY = 0.35;
const BLUR_ALPHA = 0.58;

// The smears are drawn on a layer of their own and laid on the frame in one
// draw: Chromium's canvas rounds each draw to 8 bits, so dozens of faint
// copies drawn straight onto the frame would drift its colour (red falls to
// 0 under copies below about 1/200 alpha) instead of smearing it.
let smearLayer: HTMLCanvasElement | null = null;
function layerFor(ctx: CanvasRenderingContext2D): CanvasRenderingContext2D {
  const { width, height } = ctx.canvas;
  smearLayer ??= makeCanvas(width, height);
  if (smearLayer.width !== width || smearLayer.height !== height) {
    smearLayer.width = width;
    smearLayer.height = height;
  }
  const l = smearLayer.getContext("2d")!;
  l.setTransform(1, 0, 0, 1, 0, 0);
  l.globalAlpha = 1;
  l.globalCompositeOperation = "source-over";
  l.clearRect(0, 0, width, height);
  l.setTransform(ctx.getTransform());
  return l;
}

/**
 * The motion blur of every bar in the fast middle of its flight, under the
 * bars: copies close enough that each cap's edge is a step too small to see,
 * so the smear is smooth rather than a row of ghosts, oldest and faintest
 * first. The weight is shared among them, so it is as dense however many
 * there are.
 */
function drawSmears(ctx: CanvasRenderingContext2D, lt: number): void {
  let l: CanvasRenderingContext2D | null = null;
  for (let i = 3; i >= 0; i--) {
    if (lt <= LIFT[i] || lt >= LAND[i]) continue;
    const trail = trailAt(progress(LIFT[i], LAND[i], lt));
    if (!(trail > 0)) continue;
    const now = poseAt(i, lt);
    const then = poseAt(i, lt - BLUR_SPAN);
    const travel = Math.max(Math.hypot(now.a.x - then.a.x, now.a.y - then.a.y), Math.hypot(now.b.x - then.b.x, now.b.y - then.b.y));
    const n = Math.round(clamp(Math.ceil(travel / BLUR_STEP), BLUR_MIN, BLUR_MAX));
    const share = (BLUR_COPY * trail * BLUR_MIN) / n;
    l ??= layerFor(ctx);
    for (let k = n; k >= 1; k--) {
      // A copy under one 8-bit step would only round the layer down.
      const alpha = share * (1 - k / (n + 1));
      if (alpha < 1 / 255) continue;
      const g = poseAt(i, lt - (k / n) * BLUR_SPAN);
      drawCapsule(l, g.a, g.b, g.width, PALETTE.logo, alpha);
    }
  }
  if (!l || !smearLayer) return;
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha = BLUR_ALPHA;
  ctx.drawImage(smearLayer, 0, 0);
  ctx.restore();
}

/** Bar i from its lift until it is still on its stroke. */
function drawMoving(ctx: CanvasRenderingContext2D, i: number, lt: number): void {
  const pose = poseAt(i, lt);
  drawCapsule(ctx, pose.a, pose.b, pose.width, PALETTE.logo, 1);
}

/** A soft light stretched along a stroke: glow()'s sprite, scaled out along it. */
function streak(ctx: CanvasRenderingContext2D, pose: Pose, r: number, color: string, alpha: number): void {
  if (!(alpha > 0)) return;
  const m = mid(pose);
  ctx.save();
  ctx.translate(m.x, m.y);
  ctx.rotate(angleOf(pose));
  ctx.scale((lengthOf(pose) + 2 * r) / (2 * r), 1);
  glow(ctx, 0, 0, r, color, alpha);
  ctx.restore();
}

/** Every light is out by STILL. */
const lightsOut = (lt: number): number => 1 - smoothstep(b(2.75), STILL, lt);

/** Light along each bar as it lands. */
function drawLandings(ctx: CanvasRenderingContext2D, lt: number): void {
  const out = lightsOut(lt);
  for (let i = 0; i < 4; i++) streak(ctx, TARGET[i], 110, PALETTE.logo, 0.4 * swell(lt, LAND[i], b(0.125), 0.14) * out);
}

/** Light at the two pen tips while they write, gone as they lift. */
function drawTips(ctx: CanvasRenderingContext2D, lt: number): void {
  if (lt <= PEN[0] || lt >= PEN[1]) return;
  const s = progress(PEN[0], PEN[1], lt);
  const k = smoothstep(0, 0.08, s) * (1 - smoothstep(0.7, 1, s));
  if (k <= 0) return;
  for (const [i, p] of [
    [STROKE.hShoulder, shoulderAt(lt)],
    [STROKE.kLeg, legAt(lt)],
  ] as const) {
    const tip = strokeTip(i, p);
    const at = logoToPx(LOGO_END, tip.x, tip.y);
    glow(ctx, at.x, at.y, 90, PALETTE.logo, 0.35 * k);
    glow(ctx, at.x, at.y, 34, PALETTE.glint, 0.5 * k);
  }
}

/** The mark's optical middle, px. */
const MARK_MID = logoToPx(LOGO_END, 80, 62);
/** Past the frame's farthest corner from it. */
const REACH = 1700;

/** The stage: bg dims to night from the edges in, so the light pools where the mark forms. */
function drawStage(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  const edge = 1 - smoothstep(0, b(2.25), lt);
  const core = 1 - smoothstep(b(0.75), b(3), lt);
  ctx.fillStyle = PALETTE.night;
  ctx.fillRect(0, 0, env.W, env.H);
  if (core <= 0) return;
  const g = ctx.createRadialGradient(MARK_MID.x, MARK_MID.y, 0, MARK_MID.x, MARK_MID.y, REACH);
  for (let j = 0; j <= 8; j++) g.addColorStop(j / 8, rgba(PALETTE.bg, lerp(core, edge, smoothstep(0, 1, j / 8))));
  ctx.fillStyle = g;
  ctx.fillRect(0, 0, env.W, env.H);
}

// A deep teal light behind the mark on the second heartbeat. glow()'s small
// sprite bands when it is stretched this far, so it has its own.
let backSprite: HTMLCanvasElement | null = null;
function backlight(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, alpha: number): void {
  if (!(alpha > 0)) return;
  if (!backSprite) {
    const n = 256;
    backSprite = makeCanvas(n, n);
    const g = backSprite.getContext("2d")!;
    const grad = g.createRadialGradient(n / 2, n / 2, 0, n / 2, n / 2, n / 2);
    for (let i = 0; i <= 16; i++) {
      const k = i / 16;
      grad.addColorStop(k, rgba(PALETTE.cyanDeep, Math.exp(-4 * k * k) * (1 - k * k)));
    }
    g.fillStyle = grad;
    g.fillRect(0, 0, n, n);
  }
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  ctx.globalAlpha *= clamp(alpha);
  ctx.drawImage(backSprite, x - r, y - r, r * 2, r * 2);
  ctx.restore();
}

/** A swell into `at` over `attack` and an exponential fall after, seconds. */
const swell = (lt: number, at: number, attack: number, decay: number): number =>
  lt < at ? smoothstep(at - attack, at, lt) : Math.exp(-(lt - at) / decay);

/** The second heartbeat, as light: swelling into the lub and the dub, out by STILL. */
const heart = (lt: number): number => (swell(lt, LUB[1], b(0.25), 0.2) + 0.6 * swell(lt, DUB[1], b(0.15), 0.28)) * lightsOut(lt);

/**
 * The screen the vignette spares: morph|end's, if it has one, lifting in as
 * the mark forms (b1.5 to STILL), so the vignette never shades the k darker
 * than the h once the logo is up. race|morph has none, and neither has
 * morph|end yet, so for now this is always null.
 */
function litAt(lt: number): LitRect | null {
  const end = HANDOFFS["morph|end"].lit;
  const k = smoothstep(b(1.5), STILL, lt);
  return end && k > 0 ? { ...end, alpha: end.alpha * k } : null;
}

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    // The inherited frame, exactly: four capsules on bg.
    if (lt <= 0) {
      drawHandoff(ctx, "race|morph", env);
      return;
    }
    // Set: the wordmark minus its barb on night, still until end's downbeat.
    if (lt >= STILL) {
      drawHandoff(ctx, "morph|end", env);
      return;
    }
    drawStage(ctx, lt, env);
    backlight(ctx, MARK_MID.x, MARK_MID.y, 640, 0.3 * heart(lt));
    drawLandings(ctx, lt);

    // The strokes that are still, and the two the pens are writing.
    const p = [0, 0, 0, 0, 0, 0];
    for (let i = 0; i < 3; i++) if (lt >= SETTLE[i]) p[BECOMES[i]] = 1;
    if (lt >= SWAP) {
      p[STROKE.hShoulder] = shoulderAt(lt);
      p[STROKE.kLeg] = legAt(lt);
    }
    drawLogo(ctx, LOGO_END, p);

    // The bars still waiting or moving, over their smears, the last to go underneath.
    drawSmears(ctx, lt);
    for (let i = 3; i >= 0; i--) {
      if (lt >= SETTLE[i]) continue;
      if (lt < LIFT[i]) drawWaiting(ctx, i, lt);
      else drawMoving(ctx, i, lt);
    }
    drawSparks(ctx, lt);
    drawTips(ctx, lt);
  },
  lit: litAt,
  captions: () => CAPTIONS,
};
