// A commit hk can't fix (storyboard §6.7), the poster's section. The main
// line pans left to make room for the next commit's empty slot, and a small
// terminal replays hk's real blocked run (canon80 `blocked`, frames 0–13)
// while a commit card for scripts/deploy.sh rides in along the line and
// brakes. On b3 shellcheck fails: the card's edge goes red, it grows a
// badge and shellcheck's own caret line. hk's hook whips down, snags the
// card's loop, tugs twice (heave, ho) and yanks it off the line; it swings,
// damped, from an anchor above the frame, while a red ✗ over the empty slot
// says there is no commit. On b10 the line reels it out of the top and the
// hk.pkl chip comes up for `everywhere`.
//
// Frames are pure functions of time. The first frame is restore|catch's
// (the main line with its labels) and from b11.75 the frame is
// catch|everywhere's (the chip alone). The poster is b8.5 (reel.ts
// POSTER_TIME), at an apex of the swing, so the card hangs still.

import { BEAT, type LitRect, PALETTE, type Scene, type SceneEnv, sec } from "../bible";
import { rgba } from "../color";
import { glow, ring, roundedRect, shake } from "../fx";
import { drawHandoff } from "../handoff";
import { drawChip, drawPanel } from "../kit/card";
import { drawIconHook, hookPlace, sparkle } from "../kit/logo";
import { drawMain, MAIN } from "../kit/mainline";
import { bump, land, popIn } from "../kit/motion";
import { blocked } from "../kit/screens";
import { advance, drawTerm, drawWindow, INSET, type Pane, RADIUS, termLayout, termLit } from "../kit/term";
import { clamp, DEG, inCubic, lerp, outCubic, outQuad, progress, pulse, smoothstep, spring, swiftInOut, swiftOut, TAU } from "../math";
import { type Caption, drawText, font, MONO } from "../type";
import {
  BADGE_R,
  badgeAt,
  CARD_BELOW_EYE,
  CARD_BIG,
  CARD_SMALL,
  type CardLook,
  drawCommitCard,
  drawCross,
  drawFishingLine,
  drawLoopFront,
  EYE_BOWL,
  HOOK_SCALE,
  HOOK_TILT,
  hookGlint,
  LOOP_R,
  loopCentre,
  puff,
  softShadow,
  TIE,
} from "./catch-rig";
import { BEATS, FRAME_BEATS, swingAt } from "./catch-timing";

export { BEATS, FRAME_BEATS, swingAt };

const S = sec("catch");
const b = (n: number): number => n * BEAT;

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 3 + 4 = 7 words need 4.5 beats; held 5 from b5 (line 2 needs 3, holds 4).
  { out: 10, lines: [{ in: 5, text: "Can't be fixed?" }, { in: 6, text: "hk blocks the commit." }] },
];

// The beats, section-local seconds, from scenes/catch-timing.ts, which the
// score (score/catch.ts) reads too.

/** The main line has panned. */
const PAN_END = b(BEATS.panned);
/** The inset fades in. */
const INSET_IN = b(BEATS.insetIn);
/** The card enters at the left edge, cruises, brakes from BRAKE and stops on STOP, the ✗. */
const RIDE = b(BEATS.ride);
const BRAKE = b(BEATS.brake);
export const STOP = b(BEATS.stop);
/** The hook whips down, and snags the loop. */
const DROP = b(BEATS.drop[0]);
const DROP_END = b(BEATS.drop[1]);
export const SNAG = b(BEATS.snag);
/** Heave, ho: two tugs, each over a sixteenth. */
const TUGS = [b(BEATS.tugs[0]), b(BEATS.tugs[1])] as const;
const TUG_LEN = b(BEATS.tugLen);
/** The yank lifts the card off the line; it hangs from SWING. */
const YANK = b(BEATS.yank);
export const SWING = b(BEATS.swing);
/** The line reels the card out of the top. */
const REEL = b(BEATS.reel[0]);
const REEL_END = b(BEATS.reel[1]);
/** Light runs down the hook as it holds the card up, as it ran down the logo's leg in `open`. */
const GLINT = b(BEATS.glint);
/**
 * The chip comes up, and the frame is the handoff from HOLD. It starts once
 * the reeled card, its shadow too, has cleared the chip's top (y 120).
 */
const CHIP = b(BEATS.chip);
const HOLD = b(BEATS.hold);

/** blocked frames 0–13, each shown from its beat until the next (storyboard §6.7 table). */
const FRAME_AT = FRAME_BEATS.map(b);
/** The `✗ shellcheck` row, from frame 11. */
const FAIL_ROW = 3;

// Layout.

/** The main line slides left so its dots sit at x 200 and 600. */
const PAN = -800;
/** The next commit's slot, where the line ends up. */
const SLOT_X = 1560;
/** The card rides with its bottom here, 8 px above the line. */
const CARD_FLOOR = 592;
/** Where the card stops and hangs. */
const CX = 1240;
const HANG_CY = 340;
/** The line's anchor, above the frame: the pendulum's pivot. */
const ANCHOR = { x: CX, y: -300 } as const;
/** From the rail to the hang, px. */
const LIFT = CARD_FLOOR - CARD_BIG.h - (HANG_CY - CARD_BIG.h / 2);
/** How far the line reels the card up, px: its lowest ink is above the frame by REEL_END. */
const REEL_UP = 640;
/** The card enters from here, wholly off the left edge. */
const X_FROM = -320;

// The ride: a cruise from the edge, then an even brake into the stop. The
// cruise's end is chosen so the speed is continuous where the brake starts.
const X_BRAKE = (2 * CX * (BRAKE - RIDE) + X_FROM * (STOP - BRAKE)) / (STOP - BRAKE + 2 * (BRAKE - RIDE));
const CRUISE = (X_BRAKE - X_FROM) / (BRAKE - RIDE);

function rideX(lt: number): number {
  if (lt < BRAKE) return lerp(X_FROM, X_BRAKE, progress(RIDE, BRAKE, lt));
  return lerp(X_BRAKE, CX, outQuad(progress(BRAKE, STOP, lt)));
}

function rideSpeed(lt: number): number {
  if (lt < RIDE || lt >= STOP) return 0;
  if (lt < BRAKE) return CRUISE;
  return CRUISE * (1 - progress(BRAKE, STOP, lt));
}

/** A spring's step response from `at`, 0 before. */
const step = (lt: number, at: number, freq: number, damping: number): number => (lt > at ? spring(lt - at, freq, damping) : 0);

/**
 * The card's pitch, degrees, positive nose-down (clockwise): it dips onto
 * its front corner while it brakes and rocks back past level when it stops,
 * settled to exactly 0 before the hook arrives.
 */
function pitch(lt: number): number {
  if (lt <= BRAKE || lt >= SNAG) return 0;
  return 2.4 * (step(lt, BRAKE, 2.4, 0.42) - step(lt, STOP, 2.4, 0.42)) * (1 - smoothstep(DROP, SNAG, lt));
}

/**
 * The loop's sway about its hole, degrees clockwise: thrown forward by the
 * brake, back by the stop, and upright again before the hook comes for it.
 */
function sway(lt: number): number {
  if (lt <= BRAKE || lt >= DROP_END) return 0;
  const k = step(lt, BRAKE, 2.2, 0.28) - step(lt, STOP, 2.2, 0.28) + 0.5 * step(lt, STOP, 3, 0.3) * Math.exp(-(lt - STOP) / 0.2);
  return 26 * k * (1 - smoothstep(DROP, DROP_END, lt));
}

/** One tug: the card jerks up 24 px and falls back onto the line, over a sixteenth. */
function tug(lt: number, at: number): number {
  const d = lt - at;
  if (d <= 0 || d >= TUG_LEN) return 0;
  const up = 0.035;
  if (d < up) return 24 * outQuad(d / up);
  const q = (d - up) / (TUG_LEN - up);
  return 24 * (1 - q * q);
}

/**
 * The card's bottom edge above the line, px: the snag's jolt, the tugs, the
 * yank, and the reel, which first lets the card sink a little as it takes
 * up the slack.
 */
function lift(lt: number): number {
  let y = 6 * bump(lt, SNAG + 0.03, 0.1);
  y -= 8 * bump(lt, REEL - b(0.375), b(0.5));
  y += tug(lt, TUGS[0]) + tug(lt, TUGS[1]);
  y += 4 * bump(lt, TUGS[0] + TUG_LEN, 0.09);
  y += LIFT * land(lt, YANK, SWING - YANK, 0.1);
  y += REEL_UP * inCubic(progress(REEL, REEL_END, lt));
  return y;
}

export interface Card {
  cx: number;
  bottom: number;
  w: number;
  h: number;
  pitch: number;
  shake: readonly [number, number];
  /** The card's own turn about its loop, degrees: it lags the swing a little. */
  lag: number;
  look: CardLook;
}

export function cardAt(lt: number): Card | null {
  if (lt < RIDE || lt >= REEL_END) return null;
  const grow = land(lt, STOP, 0.32, 0.14);
  const badge = popIn(lt, STOP + 0.03, 3.4, 0.38);
  const flash = pulse(lt, STOP, 0.01, 0.13);
  // Soft, breathing, once it hangs: the badge keeps the ✗ in the eye.
  const breathe = smoothstep(SWING, b(7), lt) * (0.55 + 0.2 * Math.sin((TAU * (lt - SWING)) / b(4)));
  return {
    cx: rideX(lt),
    bottom: CARD_FLOOR - lift(lt),
    w: lerp(CARD_SMALL.w, CARD_BIG.w, grow),
    h: lerp(CARD_SMALL.h, CARD_BIG.h, grow),
    pitch: pitch(lt),
    shake: shake(lt, STOP, 6, 0.07),
    lag: 0.6 * (swingAt(lt - 0.07) - swingAt(lt)),
    look: {
      red: progress(STOP, STOP + 0.05, lt),
      flash,
      warn: progress(STOP + 0.08, STOP + 0.26, lt),
      badge,
      badgeTurn: -40 * (1 - clamp(badge)),
      badgeGlow: Math.max(flash, breathe),
      shadow: 1,
      sway: sway(lt),
    },
  };
}

// The hook.

interface Hook {
  /** The eye, in the rig's frame (before the swing turns it). */
  x: number;
  y: number;
  /** Turn about the eye on top of the loaded tilt, degrees. */
  turn: number;
  /** The loop drawn in front of the stroke, 0..1: linked. */
  linked: number;
  ripple: number;
  phase: number;
}

/** The eye's y when the loop hangs in the bowl of a card whose top is `top`. */
const eyeFor = (top: number): number => top - CARD_BELOW_EYE;
/** The drop starts with the hook wholly above the frame. */
const EYE_FROM = -300;
/** The hook overshoots below the loop and comes back up onto it at the snag. */
const DIP = 22;
const SLIDE = 16;

function hookAt(lt: number, card: Card): Hook | null {
  if (lt < DROP) return null;
  const top = card.bottom - card.h;
  if (lt < SNAG) {
    const p = progress(DROP, DROP_END, lt);
    const rail = eyeFor(CARD_FLOOR - CARD_BIG.h);
    return {
      x: card.cx + SLIDE,
      y: lerp(EYE_FROM, rail + DIP, land(lt, DROP, DROP_END - DROP, 0.03)) + 2 * Math.sin(TAU * 8 * Math.max(0, lt - DROP_END)),
      turn: 12 * (1 - outCubic(p)),
      linked: 0,
      ripple: 30 * (1 - p) ** 1.3,
      phase: (lt - DROP) * 7,
    };
  }
  // Snagged: the bowl scoops the loop up and to the left, and then rides it.
  const k = outQuad(progress(SNAG, SNAG + 0.07, lt));
  return {
    x: card.cx + SLIDE * (1 - k),
    y: eyeFor(top) + DIP * (1 - k),
    turn: 0,
    linked: progress(SNAG + 0.02, SNAG + 0.07, lt),
    ripple: 0,
    phase: 0,
  };
}

/** The taut line's twang, px at its middle: plucked by the snag, the tugs and the yank. */
function twang(lt: number): number {
  const plucks: readonly (readonly [number, number])[] = [
    [SNAG + 0.02, 7],
    [TUGS[0], 4],
    [TUGS[1], 4],
    [YANK, 5],
  ];
  let v = 0;
  for (const [at, amp] of plucks) {
    const d = lt - at;
    if (d > 0 && d < 0.6) v += amp * Math.exp(-d / 0.09) * Math.sin(TAU * 15 * d);
  }
  return v;
}

// The pieces.

const pan = (lt: number): number => PAN * swiftInOut(progress(0, PAN_END, lt));
/**
 * As the card is reeled out, the view tilts up after it a little: the
 * stage drifts down, the main line (nearer) more than the terminal.
 */
const follow = (lt: number): number => smoothstep(REEL, REEL_END, lt);
/** Device pixels per px, vertically, where the scene is drawing. */
const dprOf = (ctx: CanvasRenderingContext2D): number => ctx.getTransform().d || 1;
/** `v` px rounded to whole device pixels: text is set on whole pixel rows. */
const snap = (ctx: CanvasRenderingContext2D, v: number): number => Math.round(v * dprOf(ctx)) / dprOf(ctx);
const mainAlpha = (lt: number): number => 1 - smoothstep(b(10.5), b(11.5), lt);

/** The `main` label's mono size, and its left edge once it sits centred over the panned head. */
const LABEL_SIZE = 40;
const LABEL_TO = MAIN.head.x + PAN - (advance(LABEL_SIZE) * MAIN.label.text.length) / 2;

/**
 * The `main` label's left edge: at the line's start on b0 (restore|catch),
 * gliding with the pan to sit centred over ada2ca4 by b1, where the parent
 * would otherwise come to rest right under it. The branch still names the
 * head: the commit never lands.
 */
export const labelX = (lt: number): number => lerp(MAIN.label.x, LABEL_TO, swiftInOut(progress(0, PAN_END, lt)));

/**
 * The main line. The history slides away and the `main` label follows its
 * head; the hashes and the message fade as they go. The next commit's slot
 * comes in with the line, its dashes creeping round.
 */
function drawBranch(ctx: CanvasRenderingContext2D, lt: number): void {
  const a = mainAlpha(lt);
  if (a <= 0) return;
  const p = pan(lt);
  const labels = 1 - smoothstep(0, b(0.75), lt);
  ctx.save();
  // On whole device pixels, so the `main` label (set on whole pixel rows)
  // drifts with the line rather than stepping against it.
  ctx.translate(0, snap(ctx, 40 * follow(lt)));
  drawPanTrails(ctx, lt, a);
  drawMain(ctx, { pan: p, labels: 0, alpha: a });
  ctx.save();
  ctx.globalAlpha *= a;
  const mono = font(LABEL_SIZE, 400, MONO);
  drawText(ctx, MAIN.label.text, labelX(lt), MAIN.label.y, { font: mono, fill: rgba(PALETTE.cyan, 1) });
  if (labels > 0) {
    drawText(ctx, MAIN.parent.hash, MAIN.parent.x + p, MAIN.parent.hashY, { font: mono, fill: rgba(PALETTE.text2, labels), align: "center" });
    drawText(ctx, MAIN.head.hash, MAIN.head.x + p, MAIN.head.hashY, { font: mono, fill: rgba(PALETTE.text1, labels), align: "center" });
    drawText(ctx, MAIN.head.message, MAIN.head.x + p, MAIN.head.messageY, { font: font(40, 500), fill: rgba(PALETTE.text2, labels), align: "center" });
  }
  // The slot. It comes in off the line's fixed right end (x 1760), fading
  // up as it reaches it, so it never floats in the space past the line.
  const sx = SLOT_X + p - PAN;
  const onLine = clamp((MAIN.x1 + MAIN.dotR - sx) / 100);
  if (onLine > 0) {
    ctx.save();
    ctx.globalAlpha *= onLine;
    ctx.fillStyle = PALETTE.bg;
    ctx.beginPath();
    ctx.arc(sx, MAIN.y, MAIN.dotR, 0, TAU);
    ctx.fill();
    ctx.setLineDash([6, 6]);
    ctx.lineDashOffset = -lt * 9;
    ctx.strokeStyle = PALETTE.text3;
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.restore();
  }
  // No commit: a red ✗ flashes over the slot on b6 and glows on above it,
  // the slot still dashed and empty under it.
  const pop = popIn(lt, SWING, 4, 0.42);
  if (pop > 0) {
    const flash = pulse(lt, SWING, 0.012, 0.16);
    const after = lerp(1, 0.85, smoothstep(SWING + 0.15, b(7.5), lt));
    const y = MAIN.y - 58;
    glow(ctx, SLOT_X, y, 60 + 50 * flash, PALETTE.red, 0.28 + 0.6 * flash);
    ring(ctx, SLOT_X, y, 80, progress(SWING, SWING + 0.4, lt), PALETTE.red, 4);
    ctx.save();
    ctx.globalAlpha *= after;
    ctx.translate(SLOT_X, y);
    ctx.scale(pop, pop);
    drawCross(ctx, 0, 0, 44, PALETTE.red, 0.13);
    ctx.restore();
  }
  ctx.restore();
  ctx.restore();
}

/**
 * The pan is quick: the dots leave a short trail, what a 1/60 s shutter
 * would catch, so they read as moving rather than jumping at 60 fps.
 */
function drawPanTrails(ctx: CanvasRenderingContext2D, lt: number, alpha: number): void {
  const h = 1 / 480;
  const v = (pan(lt + h) - pan(lt - h)) / (2 * h);
  const trail = -v / 60;
  // Faded in with its length, so it never starts whole.
  const k = clamp((trail - 2) / 30);
  if (k <= 0) return;
  const p = pan(lt);
  ctx.save();
  ctx.globalAlpha *= alpha * k;
  ctx.lineCap = "round";
  for (const [x, color] of [
    [MAIN.parent.x + p, PALETTE.text3],
    [MAIN.head.x + p, PALETTE.cyan],
  ] as const) {
    const g = ctx.createLinearGradient(x, 0, x + trail, 0);
    g.addColorStop(0, rgba(color, 0.55));
    g.addColorStop(1, rgba(color, 0));
    ctx.strokeStyle = g;
    ctx.lineWidth = 2 * MAIN.dotR * 0.85;
    ctx.beginPath();
    ctx.moveTo(x, MAIN.y);
    ctx.lineTo(x + trail, MAIN.y);
    ctx.stroke();
  }
  ctx.restore();
}

/**
 * The inset: kit/term's INSET (x 120–880 from y 110) cut down to the 7 rows
 * a blocked frame fills at most, its bottom edge at y 390 rather than 430,
 * 27 px under the last baseline as the first sits 37 px under its top. The
 * card's loop, sized to take the hook (catch-rig LOOP_R), tops out at y 405
 * as the card rides in beneath the pane: it clears the edge by 15 px rather
 * than crossing it.
 */
export const PANE: Pane = { ...INSET, h: 280, rows: 7 };

/**
 * The inset's fade, and where it stands: it rises a little as it comes, and
 * drifts down after the reeled card as it goes, on whole device pixels
 * (`dpr` of them to the px) so its text drifts with its window.
 */
export function insetPane(lt: number, dpr = 1): { pane: Pane; alpha: number } {
  const come = swiftOut(progress(INSET_IN, b(1), lt));
  const go = smoothstep(b(10), b(11), lt);
  const dy = 16 * (1 - come) + Math.round(20 * follow(lt) * dpr) / dpr;
  return { pane: { ...PANE, y: PANE.y + dy, baseline0: PANE.baseline0 + dy }, alpha: come * (1 - go) };
}

/**
 * The inset's shadow, falling mostly below it: the card's loop rides in
 * through it, just under the pane's bottom edge, and the shade on it says
 * the terminal is in front. Only outside the pane, which would otherwise
 * darken through its window as it fades.
 */
function drawInsetShadow(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  const { pane, alpha } = insetPane(lt, dprOf(ctx));
  if (alpha <= 0) return;
  ctx.save();
  roundedRect(ctx, pane.x, pane.y, pane.w, pane.h, RADIUS);
  ctx.rect(0, 0, env.W, env.H);
  ctx.clip("evenodd");
  softShadow(ctx, pane, { dy: 8, spread: 24, alpha: 0.55 * alpha, radius: RADIUS });
  ctx.restore();
}

/** Which blocked frame is up, or −1 before the first. */
function frameAt(lt: number): number {
  let f = -1;
  for (let i = 0; i < FRAME_AT.length; i++) if (lt >= FRAME_AT[i] - 1e-9) f = i;
  return f;
}

/** hk's blocked run, frame by frame: a new or changed row fades in over a 64th. */
function drawInset(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  const { pane, alpha } = insetPane(lt, dprOf(ctx));
  if (alpha <= 0) return;
  ctx.save();
  ctx.globalAlpha *= alpha;
  drawWindow(ctx, pane, false);
  ctx.restore();
  const f = frameAt(lt);
  if (f < 0) return;
  const lines = blocked[f];
  const prev: readonly string[] = f > 0 ? blocked[f - 1] : [];
  const at = FRAME_AT[f];
  if (f >= 11) {
    // The failing row, marked red from the moment it fails.
    const L = termLayout(pane, lines.length);
    const flash = pulse(lt, STOP, 0.01, 0.16);
    const sweep = swiftOut(progress(STOP, STOP + 0.12, lt));
    const y = L.baseline(FAIL_ROW) - 0.8 * pane.size - (pane.lineH - pane.size) / 2;
    ctx.save();
    ctx.globalAlpha *= alpha;
    ctx.fillStyle = rgba(PALETTE.red, 0.13 + 0.3 * flash);
    ctx.fillRect(pane.x + 6, y, (pane.w - 12) * sweep, pane.lineH);
    ctx.restore();
  }
  // hk redraws its screen in place: a row whose text changed crossfades over
  // a 64th, the old text going as the new comes (no frame scrolls: 7 rows at
  // most in a pane of 8).
  const k = progress(at, at + BEAT / 16, lt);
  const text = { ...pane, window: false };
  if (k < 1 && prev.length) {
    drawTerm(ctx, text, prev, { t: env.t, alpha, lineAlpha: (i) => (prev[i] === lines[i] ? 0 : 1 - k) });
  }
  drawTerm(ctx, text, lines, { t: env.t, alpha, lineAlpha: (i) => (prev[i] === lines[i] ? 1 : k) });
}

/**
 * Speed lines behind the card while it cruises, and the ghosts a 1/60 s
 * shutter would leave, gone as it brakes; and lines under it as it is
 * yanked up off the line.
 */
function drawStreaks(ctx: CanvasRenderingContext2D, card: Card, lt: number): void {
  const left = card.cx - card.w / 2;
  const top = card.bottom - card.h;
  const h = 1 / 480;
  const vy = (lift(lt + h) - lift(lt - h)) / (2 * h);
  const ky = clamp((vy - 400) / 1200) * progress(YANK, YANK + 0.04, lt) * (lt < SWING ? 1 : 0);
  if (ky > 0) {
    ctx.save();
    ctx.lineCap = "round";
    [0.3, 0.5, 0.7].forEach((f, i) => {
      const x = left + card.w * f;
      const len = vy * (0.05 + 0.02 * ((i + 1) % 3)) * ky;
      const y0 = card.bottom + 14;
      const g = ctx.createLinearGradient(0, y0, 0, y0 + len);
      g.addColorStop(0, rgba(PALETTE.text3, 0.5 * ky));
      g.addColorStop(1, rgba(PALETTE.text3, 0));
      ctx.strokeStyle = g;
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(x, y0);
      ctx.lineTo(x, y0 + len);
      ctx.stroke();
    });
    ctx.restore();
  }
  const v = rideSpeed(lt);
  const k = clamp((v - 300) / 1500);
  if (k <= 0) return;
  // Ghosts: the card's panel where it was over the last 60th of a second.
  for (let i = 3; i >= 1; i--) {
    const dx = (v / 60) * (i / 3);
    ctx.save();
    ctx.globalAlpha *= 0.22 * k * (1 - i / 4);
    drawPanel(ctx, { x: left - dx, y: top, w: card.w, h: card.h }, { fill: PALETTE.surface, stroke: PALETTE.divider });
    ctx.restore();
  }
  ctx.save();
  ctx.lineCap = "round";
  [0.28, 0.52, 0.78].forEach((f, i) => {
    const len = v * (0.07 + 0.03 * i) * k;
    const y = top + card.h * f;
    const g = ctx.createLinearGradient(left - 12 - len, 0, left - 12, 0);
    g.addColorStop(0, rgba(PALETTE.text3, 0));
    g.addColorStop(1, rgba(PALETTE.text3, 0.45 * k));
    ctx.strokeStyle = g;
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.moveTo(left - 12 - len, y);
    ctx.lineTo(left - 12, y);
    ctx.stroke();
  });
  ctx.restore();
}

/** The card's shadow on the line, gone as it lifts. */
function drawContact(ctx: CanvasRenderingContext2D, card: Card): void {
  const up = CARD_FLOOR - card.bottom;
  const a = 0.5 * (1 - clamp(up / 70));
  if (a <= 0) return;
  ctx.save();
  ctx.globalAlpha = a;
  ctx.fillStyle = "#000";
  const w = card.w * (0.5 - 0.1 * clamp(up / 70));
  for (let i = 0; i < 4; i++) {
    ctx.globalAlpha = a * 0.3;
    ctx.beginPath();
    ctx.ellipse(card.cx, MAIN.y + 2, w * (0.8 + 0.12 * i), 5 + 3 * i, 0, 0, TAU);
    ctx.fill();
  }
  ctx.restore();
}

/** Dust where the card touches the line hard: the stop and the tugs' landings. */
function drawDust(ctx: CanvasRenderingContext2D, card: Card, lt: number): void {
  const l = card.cx - card.w / 2;
  const r = card.cx + card.w / 2;
  const y = CARD_FLOOR + 2;
  puff(ctx, r - 6, y, 1, progress(STOP - 0.02, STOP + 0.4, lt), 11, 6, 44);
  for (const [i, at] of [TUGS[0] + TUG_LEN, TUGS[1] + TUG_LEN].entries()) {
    const u = progress(at, at + 0.35, lt);
    puff(ctx, l + 8, y, -1, u, 20 + i);
    puff(ctx, r - 8, y, 1, u, 30 + i);
  }
}

/**
 * The line, the card and the hook, turned together by the swing about the
 * anchor above the frame.
 */
function drawRig(ctx: CanvasRenderingContext2D, lt: number): void {
  const found = cardAt(lt);
  if (!found) return;
  const theta = swingAt(lt);
  // Upright, the card moves on whole device pixels up and down: text is set
  // on whole pixel rows, and would otherwise step against its panel as the
  // card bobs on the line.
  const upright = !theta && !found.pitch && !found.lag;
  const card = upright ? { ...found, bottom: snap(ctx, found.bottom) } : found;
  drawStreaks(ctx, card, lt);
  drawContact(ctx, card);
  drawDust(ctx, card, lt);
  // A pool of light where the card hangs, so it stands off the stage.
  const pool = smoothstep(YANK, b(6.5), lt) * (1 - smoothstep(REEL, b(10.75), lt));
  // Its edge stays above the captions' band.
  if (pool > 0) glow(ctx, CX, HANG_CY - 40, 420, PALETTE.cyanDeep, 0.2 * pool);
  const hook = hookAt(lt, card);
  const rect = { x: card.cx - card.w / 2, y: card.bottom - card.h, w: card.w, h: card.h };
  const loop = loopCentre(card.cx, rect.y);

  ctx.save();
  if (theta) {
    ctx.translate(ANCHOR.x, ANCHOR.y);
    ctx.rotate(theta * DEG);
    ctx.translate(-ANCHOR.x, -ANCHOR.y);
  }
  if (hook) {
    drawFishingLine(ctx, ANCHOR, { x: hook.x, y: hook.y - TIE }, { ripple: hook.ripple, phase: hook.phase, twang: twang(lt) });
  }

  // The card, in its own frame: shaken by the ✗, pitched on a corner by the
  // brake, and lagging the swing about its loop.
  ctx.save();
  ctx.translate(card.shake[0], card.shake[1]);
  if (card.pitch) {
    const px = card.pitch > 0 ? rect.x + rect.w : rect.x;
    ctx.translate(px, card.bottom);
    ctx.rotate(card.pitch * DEG);
    ctx.translate(-px, -card.bottom);
  }
  if (card.lag) {
    ctx.translate(loop.x, loop.y);
    ctx.rotate(card.lag * DEG);
    ctx.translate(-loop.x, -loop.y);
  }
  drawCommitCard(ctx, rect, card.look);
  // The ✗'s shock ring, off the badge.
  const bc = badgeAt(rect.x + rect.w, rect.y);
  ring(ctx, bc.x, bc.y, BADGE_R * 3.4, progress(STOP + 0.02, STOP + 0.45, lt), PALETTE.red, 5);
  ctx.restore();

  if (hook) {
    // hk's own mark, lit a little from within.
    glow(ctx, hook.x, hook.y + EYE_BOWL * 0.8, 150, PALETTE.logo, 0.1);
    const place = hookPlace(hook.x, hook.y, HOOK_SCALE);
    drawIconHook(ctx, place, 0, { rotate: HOOK_TILT + hook.turn, barb: 1 });
    hookGlint(ctx, place, HOOK_TILT + hook.turn, progress(GLINT, GLINT + b(1.5), lt));
    drawLoopFront(ctx, loop, hook.linked);
    const s = progress(SNAG + 0.02, SNAG + 0.3, lt);
    if (s > 0 && s < 1) sparkle(ctx, loop.x + 12, loop.y - LOOP_R - 2, s);
  }
  ctx.restore();
}

function draw(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  if (lt >= HOLD) {
    drawHandoff(ctx, "catch|everywhere", env);
    return;
  }
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, env.W, env.H);
  drawBranch(ctx, lt);
  if (lt >= CHIP) {
    const k = swiftOut(progress(CHIP, HOLD, lt));
    drawChip(ctx, { alpha: k, scale: lerp(0.92, 1, k) });
  }
  drawRig(ctx, lt);
  // The terminal sits in front: its shadow falls across the card's loop as
  // the card rides in under it.
  drawInsetShadow(ctx, lt, env);
  drawInset(ctx, lt, env);
}

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    ctx.save();
    draw(ctx, lt, env);
    ctx.restore();
  },
  // The inset is a terminal: the vignette spares it while it is up.
  lit(lt): LitRect | null {
    const { pane, alpha } = insetPane(lt);
    return alpha > 0 ? termLit(pane, alpha) : null;
  },
  captions: () => CAPTIONS,
};
