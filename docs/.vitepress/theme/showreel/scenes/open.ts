// Section 1, "hk" (storyboard §6.1): on the night stage the favicon's hook
// hangs on its line from the top edge, bobbing, while a lit pen waits at the
// top of the h over the wordmark's dotted guide. The pen inks the h and the
// k stroke by stroke on the beat as the hook is lowered to the k and bounces
// to rest on its dock (b3). The pen draws the k's leg out of the arm, round
// the hook's bowl and up to its point while the hook docks into it, and the
// line lets go and whips up out of frame. On the next downbeat (b4) the barb
// clicks out of the point, a glint runs down the leg into a sparkle, and the
// leg swings on its root and settles; the stage cross-fades from night to
// bg and the wordmark rests alone on the open|config frame.

import { BAR, BEAT, PALETTE, type Scene, sec } from "../bible";
import { mix, rgba } from "../color";
import { glow } from "../fx";
import { drawHandoff } from "../handoff";
import {
  drawIconHook,
  drawLogo,
  glint,
  hookAnchors,
  LOGO_OPEN,
  LOGO_POINT,
  LOGO_STROKES,
  logoToPx,
  settledSwing,
  sparkle,
  STROKE,
} from "../kit/logo";
import { bump, jolt } from "../kit/motion";
import { clamp, cubicBezier, hash, inOutSine, inQuad, keys, outQuad, progress, pulse, smoothstep, TAU } from "../math";
import type { Caption } from "../type";
import { drawGuide, drawHotInk, drawPenLight, drawWhipLine, penAt, type Window } from "./open-pen";

const S = sec("open");

/** Local time of beat `n` of the section. */
const b = (n: number): number => n * BEAT;

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 5 + 2 = 7 words need 4.5 beats; held 4.5 from b2.5 (line 2 needs 2, holds 4).
  { out: 7, lines: [{ in: 2.5, text: "Git hooks and project checks," }, { in: 3, text: "in parallel." }] },
];

// The beat map, local seconds. The score (score/open.ts) is written to these.

/**
 * Each stroke's window, in LOGO_STROKES order: hStem b0–⅔, hShoulder b1–1.5,
 * kStem b2–2⅓, kArm b2.5–2.8, kLeg b3–3.75 (the dock), kBarb b4–4.125 (the click).
 */
export const STROKE_AT: readonly Window[] = [
  [b(0), b(2 / 3)],
  [b(1), b(1.5)],
  [b(2), b(2 + 1 / 3)],
  [b(2.5), b(2.8)],
  [b(3), b(3.75)],
  [b(4), b(4.125)],
];
/** The hook comes to rest on its dock, and docking starts. */
export const T_DOCK = b(3);
/** The line goes slack and whips up out of frame. */
export const T_LET_GO = b(3.5);
const T_LINE_GONE = b(4);
/** The barb clicks out of the point: the reel's first hit. */
export const T_CLICK = b(4);
/** The glint runs the leg from the click to here, where the sparkle peaks. */
export const T_SPARKLE = b(4.75);
/** The leg's swing is eased out over this span and exactly still from its end. */
const CALM: readonly [number, number] = [b(5.5), b(7)];
/** The stage cross-fades from night to bg. */
const BG_FADE: readonly [number, number] = [b(6), b(8)];
/** From here every frame is the open|config handoff. */
const T_REST = b(7.875);

// The hook: its eye starts at (1090, 120) and docks at (1090, 420), where
// its bowl is the k's (LOGO_OPEN: the eye is logo (132, 68)).
const DROP = -300;
/** The line comes from just above the frame. */
const LINE_TOP = -10;

/**
 * The hook's drop: let go from its bob, it picks up speed in a tenth of a
 * second, then pays out fast and slows (the ease-out the reel's ratchet
 * clicks to), easing into a bounce past its dock on the stretch of the line
 * and a small rebound, and drops into its seat on the dock at b3. The
 * segments meet at equal speeds, so the drop has no kinks.
 */
const dropAt = keys([
  [0, DROP],
  [b(0.2), DROP + 27, inQuad],
  [b(2.3), 16, outQuad],
  [b(2.7), -5, inOutSine],
  [T_DOCK, 0, inQuad],
]);

/**
 * The hook's offset from its dock, px: the drop, plus the idle bob it
 * arrives with (global t), and the knock of the seat, which is over before
 * the leg's ink reaches the bowl.
 */
function hookDy(lt: number, t: number): number {
  if (lt >= T_DOCK) return 2 * jolt(lt, T_DOCK, 0.14, 9);
  // A 2-unit (5 px) sine with a one-bar period, as if in water, fading as the drop takes over.
  const bob = 5 * Math.sin((TAU * t) / BAR) * (1 - smoothstep(0, b(1.5), lt));
  return dropAt(lt) + bob;
}

/** A small sway about the eye as it is lowered, still by the time it docks. */
function hookTurn(lt: number): number {
  if (lt >= T_DOCK) return 0;
  return 4 * Math.sin((TAU * lt) / (BAR * 0.75)) * (1 - smoothstep(b(1), T_DOCK, lt));
}

/**
 * The docked leg draws back a couple of degrees against the swing just
 * before the click kicks it (the anticipation), and is back by b4.1.
 */
const cockAt = (lt: number): number => -2.5 * bump(lt, b(3.78), b(0.32));

/** How far the hook has docked into the k. */
const dockAt = (lt: number): number => smoothstep(T_DOCK, b(3.75), lt);

/**
 * The eye and the top of the shank are gone by the time the line lets go,
 * so its knot slips off the hook rather than leaving the eye hanging by the
 * arm while the dock finishes.
 */
const eyeAt = (lt: number): number => 1 - smoothstep(b(3.2), T_LET_GO, lt);

// The pen: each stroke fast off the mark and landing soft; the leg evenly
// round the bowl; the barb a flick.
const PEN = cubicBezier(0.25, 0.55, 0.35, 1);
const LEG = cubicBezier(0.35, 0.1, 0.4, 1);
function strokeProgress(i: number, lt: number): number {
  const [a, z] = STROKE_AT[i];
  const u = progress(a, z, lt);
  if (i === STROKE.kLeg) return LEG(u);
  if (i === STROKE.kBarb) return 1 - (1 - u) ** 2;
  return PEN(u);
}
const strokesAt = (lt: number): number[] => LOGO_STROKES.map((_, i) => strokeProgress(i, lt));

/** Fresh ink stays hot while the pen draws it and cools over 0.2 s after. */
const heatOf = (i: number, lt: number): number => {
  const [a, z] = STROKE_AT[i];
  if (lt <= a) return 0;
  return 1 - progress(z, z + 0.2, lt);
};

/** The pen is lit from the first frame, and goes out after the barb's click. */
const penAlpha = (lt: number): number => 1 - smoothstep(STROKE_AT[STROKE.kBarb][1], b(4.6), lt);

/** The dotted guide, at full strength until the ink has covered it. */
const guideAlpha = (lt: number): number => 0.45 * (1 - smoothstep(b(3.5), b(4.25), lt));

/**
 * The ink's soft halo: the fresh strokes glow as they are drawn, flare on
 * the click, and cool to hk's flat stroke before the stage turns.
 */
function haloAt(lt: number): number {
  const on = 0.4 + 0.5 * pulse(lt, T_CLICK, 0.02, 0.3);
  return on * (1 - smoothstep(b(4.5), b(6.75), lt));
}

/**
 * A cool light on the stage behind the mark, an ellipse that ends above the
 * captions' band; it swells on the click and is gone with the night. Brand
 * cyan, not logo cyan, which is the wordmark's and the hook's alone (§3).
 */
function stageLight(ctx: CanvasRenderingContext2D, lt: number, click: number): void {
  const a = (0.05 + 0.09 * click) * (1 - smoothstep(b(5), BG_FADE[1] - b(0.5), lt));
  if (!(a > 0.001)) return;
  const R = 860;
  ctx.save();
  ctx.translate(960, 400);
  ctx.scale(1, 0.38);
  const g = ctx.createRadialGradient(0, 0, 0, 0, 0, R);
  g.addColorStop(0, rgba(PALETTE.cyan, a));
  g.addColorStop(0.4, rgba(PALETTE.cyan, a * 0.42));
  g.addColorStop(0.75, rgba(PALETTE.cyan, a * 0.1));
  g.addColorStop(1, rgba(PALETTE.cyan, 0));
  ctx.fillStyle = g;
  ctx.fillRect(-R, -R, 2 * R, 2 * R);
  ctx.restore();
}

/**
 * Motes in the dark water the hook hangs in: soft, out-of-focus ones near
 * the lens rising faster than the sharp far ones, so the empty stage has
 * depth. They stay above the captions' band and go with the night. Brand
 * cyan near and its bright tint far: no logo cyan off the marks.
 */
const MOTES = Array.from({ length: 26 }, (_, i) => {
  const near = i % 3 === 0;
  return {
    near,
    x: 120 + hash(i, 71) * 1680,
    y: 60 + hash(i, 72) * 660,
    rise: (near ? 26 : 9) * (0.7 + 0.6 * hash(i, 73)),
    sway: 6 + 10 * hash(i, 74),
    phase: TAU * hash(i, 75),
    size: near ? 7 + 5 * hash(i, 76) : 1.6 + 1.1 * hash(i, 76),
    alpha: near ? 0.1 + 0.08 * hash(i, 77) : 0.22 + 0.2 * hash(i, 77),
  };
});
const MOTE_TOP = 50;
const MOTE_FLOOR = 700;
function drawMotes(ctx: CanvasRenderingContext2D, lt: number, fade: number): void {
  if (!(fade > 0)) return;
  const span = MOTE_FLOOR + 20 - (MOTE_TOP - 20);
  for (const m of MOTES) {
    // Rising and wrapping round, invisible where they wrap.
    let y = m.y - m.rise * lt;
    y = MOTE_TOP - 20 + ((((y - (MOTE_TOP - 20)) % span) + span) % span);
    const x = m.x + m.sway * Math.sin(m.phase + (TAU * lt) / 3.2);
    const edge = smoothstep(MOTE_TOP, MOTE_TOP + 60, y) * (1 - smoothstep(MOTE_FLOOR - 70, MOTE_FLOOR, y));
    const a = m.alpha * edge * fade * (0.8 + 0.2 * Math.sin(m.phase * 3 + TAU * lt * 0.7));
    if (!(a > 0.004)) continue;
    if (m.near) {
      glow(ctx, x, y, m.size * 2.2, PALETTE.cyan, a);
    } else {
      ctx.fillStyle = rgba(PALETTE.cyanBright, a);
      ctx.beginPath();
      ctx.arc(x, y, m.size, 0, TAU);
      ctx.fill();
    }
  }
}

/** Draw `paint` with a soft glow of logo cyan at `amount`, sized in device px. */
function withHalo(ctx: CanvasRenderingContext2D, amount: number, paint: () => void): void {
  if (!(amount > 0.001)) {
    paint();
    return;
  }
  const m = ctx.getTransform();
  ctx.save();
  ctx.shadowColor = rgba(PALETTE.logo, 0.6 * clamp(amount));
  ctx.shadowBlur = 34 * Math.hypot(m.a, m.b);
  paint();
  ctx.restore();
}

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    if (lt >= T_REST) {
      drawHandoff(ctx, "open|config", env);
      return;
    }
    const t = env.t;
    const place = LOGO_OPEN;
    const click = pulse(lt, T_CLICK, 0.02, 0.35) * (1 - smoothstep(b(6), b(7.5), lt));

    // The stage: night, turning to bg.
    const kb = smoothstep(BG_FADE[0], BG_FADE[1], lt);
    ctx.fillStyle = kb >= 1 ? PALETTE.bg : mix(PALETTE.night, PALETTE.bg, kb);
    ctx.fillRect(0, 0, env.W, env.H);
    stageLight(ctx, lt, click);
    drawMotes(ctx, lt, 1 - smoothstep(b(5), b(7), lt));

    drawGuide(ctx, place, guideAlpha(lt));

    // The line, taut from above the frame to the hook's eye until it lets
    // go and whips away. Drawn here rather than by drawIconHook, so it
    // takes none of the hook's halo.
    const k = dockAt(lt);
    const halo = haloAt(lt);
    const dy = hookDy(lt, t);
    if (lt < T_LINE_GONE) {
      const tie = hookAnchors(place, 0, { offset: [0, dy] }).ringTop;
      drawWhipLine(ctx, tie.x, LINE_TOP, tie.y, progress(T_LET_GO, T_LINE_GONE, lt));
    }
    // The hook, under the ink that covers it once docked; drawn until it has
    // become the tail of the leg (drawing it twice brightens the leg's
    // edges). Its halo fades as it docks, so it never doubles the leg's.
    if (k < 1) {
      const hook = { offset: [0, dy] as const, rotate: hookTurn(lt), eyeAlpha: eyeAt(lt) };
      withHalo(ctx, halo * (1 - k), () => drawIconHook(ctx, place, k, hook));
    }
    // The dock's knock: a soft light in the bowl as it seats.
    const knock = pulse(lt, T_DOCK, 0.015, 0.12);
    if (knock > 0.01) {
      const bowl = hookAnchors(place).bowl;
      glow(ctx, bowl.x, bowl.y, 120, PALETTE.logo, 0.45 * knock);
    }

    // The wordmark, the leg swinging from the click and still from b7.
    const swing = settledSwing(lt, T_CLICK, 8, BAR, CALM) + cockAt(lt);
    const p = strokesAt(lt);
    withHalo(ctx, halo, () => drawLogo(ctx, place, p, { swing }));
    for (let i = 0; i < LOGO_STROKES.length; i++) drawHotInk(ctx, place, i, p[i], heatOf(i, lt), swing);

    // The click: the barb flicks out of the point, which flashes; a glint
    // runs down the leg into a sparkle on the point, riding the swing.
    const pt = logoToPx(place, LOGO_POINT[0], LOGO_POINT[1], swing);
    const flash = pulse(lt, T_CLICK, 0.01, 0.09);
    if (flash > 0.01) glow(ctx, pt.x, pt.y, 90, PALETTE.glint, 0.55 * flash);
    glint(ctx, place, inOutSine(progress(T_CLICK, T_SPARKLE, lt)), { swing });
    sparkle(ctx, pt.x, pt.y, progress(T_SPARKLE - b(0.3), T_SPARKLE + b(0.3), lt));

    // The pen, lit from the first frame.
    const pen = penAt(lt, place, STROKE_AT, (i) => p[i], swing);
    drawPenLight(ctx, pen.at, pen.ink, penAlpha(lt) * (0.94 + 0.06 * Math.sin((TAU * t) / BEAT)));
  },
  lit: () => null,
  captions: () => CAPTIONS,
};
