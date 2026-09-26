// Unstaged work is stashed (storyboard §6.4, 16–22 s, 12 beats). hk sets the
// unstaged line aside, so the linters see only what was staged.
//
// The commit's pane squeezes into a strip at the top as the staged
// src/main.py rises and lands on b1.5, its unstaged last line marked out in
// paper with a marching dashed outline. On b2 a blade cuts along the outline
// and the line peels up behind it (stash-peel.ts), is flung in an arc into
// the stash tray, and the lid slams on b3 as the pane reports the stash. A
// cyan read band sweeps what the linters will see, the four lines ruff and
// ruff-format will change are underlined on the eighths, and the scene folds
// into the lanes: the strip leaves through the top, the card collapses into
// its file's lane label, the other labels type on, the tracks draw, and the
// padlocks pop in.

import { BEAT, PALETTE, type Scene, sec } from "../bible";
import { mix, rgba } from "../color";
import { glow, ring, roundedRect } from "../fx";
import { bg, drawHandoff } from "../handoff";
import { type CardRect, cardLayout, drawCard, drawMono, MAIN_PY_STAGED, monoWidth } from "../kit/card";
import { drawLanes, drawPadlock, LANE_FILES, LANES } from "../kit/lanes";
import { bump, land } from "../kit/motion";
import { commit, PROMPT_COMMIT } from "../kit/screens";
import { drawTerm, lerpPane, PANE_FULL, type Pane, STRIP as STRIP_PANE, termLit } from "../kit/term";
import { drawTray, TRAY } from "../kit/tray";
import { clamp, cubicBezier, DEG, hash, inOutCubic, inOutSine, lerp, outCubic, progress, smoothstep, spring, swiftIn, swiftInOut, swiftOut, TAU } from "../math";
import { type Caption, drawText, font } from "../type";
import { DASH, dashAt, drawBlade, drawSparks, drawStrip, drawStripFace, INTO, PEEL, SPARKS_AT, STRIP, stripPose } from "./stash-peel";
import { CARD_IN, CARD_OUT, HOLD, LID, LOCKS, PANE_IN, READ, ROW_AT, SETTLED, SQUIGGLE_DRAW, SQUIGGLES, STRIP_OUT, TAG_IN, TRACKS, TRAY_IN, TYPE } from "./stash-timing";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 4 + 7 = 11 words need 6.5 beats; held 6.5 from b3 (line 2 needs 4.5, holds 5.5). Line 2 is exactly 36 characters.
  { out: 9.5, lines: [{ in: 3, text: "Unstaged work is stashed," }, { in: 4, text: "so linters see only what you staged." }] },
];

const S = sec("stash");

// Beats, section-local, are stash-timing.ts's, which the score reads too.

// Layout.

/** The staged file: x 660–1260, y 284–696, 26 px mono on a 32 px pitch (baselines 356.8 … 676.8). */
const CARD: CardRect = { x: 660, y: 284, w: 600, h: 412 };
const CARD_TEXT = { size: 26, lineH: 32 } as const;
const L = cardLayout(CARD, CARD_TEXT);
const TAB = "src/main.py";
/** The card draws a tab this wide with no text; the scene sets the name itself, so it can fly into the lane label. */
const TAB_BLANK = " ".repeat(TAB.length);
/** The card's tab strip, as deep as card.ts draws it. */
const TAB_BAND = 40;
const TAB_AT = { x: CARD.x + 24, y: CARD.y + 20 + 0.3 * 30, size: 30 } as const;
/** Lane 3's label, where the tab's name lands (lanes.ts: mono 40 px text1 at x 160, baseline cy + 14). */
const LABEL_AT = { x: LANES.labelX, y: LANES.rows[2] + 14, size: 40 } as const;
/** Where the card starts, below the frame. */
const CARD_FROM_Y = 1120;
/** The DETAIL tag, on line 11's baseline. */
const TAG = { x: 1290, y: 677 } as const;

const LINES_FILES: readonly string[] = [PROMPT_COMMIT, ...commit[1]];
const LINES_STASHED: readonly string[] = [PROMPT_COMMIT, ...commit[2]];

/**
 * An analytic spring (freq Hz, damping ratio) from 0 at beat `from` that is
 * exactly 1 from beat `until`, blended home over the last three sixteenths.
 */
function springTo(b: number, from: number, until: number, freq: number, damping: number): number {
  if (b <= from) return 0;
  if (b >= until) return 1;
  const v = spring((b - from) * BEAT, freq, damping);
  return v + (1 - v) * smoothstep(until - 0.375, until, b);
}

/** How fast the card is going, in rest-heights a beat, as it is launched and as it reaches its place. */
const RISE = { launch: 1.6, arrive: 0.3 } as const;

/**
 * The card's rise at beat b: 0 below the frame, 1 at rest. Launched at
 * CARD_IN.from, it slows all the way up but is still moving when it reaches
 * its place on CARD_IN.to (a cubic Hermite, monotonic for these speeds), so
 * it lands on the beat; it carries on up a few pixels and settles home,
 * exactly 1 and still from CARD_IN.settled.
 */
export function cardRise(b: number): number {
  if (b <= CARD_IN.from) return 0;
  if (b >= CARD_IN.settled) return 1;
  if (b < CARD_IN.to) {
    const span = CARD_IN.to - CARD_IN.from;
    const u = (b - CARD_IN.from) / span;
    const m0 = RISE.launch * span;
    const m1 = RISE.arrive * span;
    return (u ** 3 - 2 * u ** 2 + u) * m0 + (3 * u ** 2 - 2 * u ** 3) + (u ** 3 - u ** 2) * m1;
  }
  // The overshoot leaves at the arrival's speed and comes home at rest.
  const T = CARD_IN.settled - CARD_IN.to;
  const s = (b - CARD_IN.to) / T;
  return 1 + ((RISE.arrive * T) / Math.PI) * Math.sin(Math.PI * s) * (1 - s) ** 2;
}

/**
 * How much the hold breathes at beat b: 0 outside HOLD, 1 across its middle.
 * The card itself stays put: text snaps to whole pixels, so a slow float of
 * a pixel or two would judder.
 */
const holdEnv = (b: number): number => smoothstep(HOLD.from, HOLD.full, b) * (1 - smoothstep(HOLD.fade, HOLD.to, b));

/** A pulse on the beats through the hold, −1..1 scaled by holdEnv: 1 on each beat. */
const holdPulse = (b: number): number => holdEnv(b) * Math.cos(TAU * b);

// The terminal.

/** The pane at beat b: PANE_FULL squeezing into STRIP (k past 1 overshoots a little), then leaving through the top. */
function paneAt(b: number): { pane: Pane; scroll: number } | null {
  if (b >= STRIP_OUT.to) return null;
  if (b < STRIP_OUT.from) {
    // Eased in from rest, a hair past STRIP, and home on b1.5.
    const k = b < 1.25 ? 1.012 * inOutCubic(progress(PANE_IN.from, 1.25, b)) : lerp(1.012, 1, inOutSine(progress(1.25, PANE_IN.to, b)));
    return { pane: k === 1 ? STRIP_PANE : lerpPane(PANE_FULL, STRIP_PANE, k), scroll: k };
  }
  // A dip, then away up and out of the frame.
  const dy = 7 * bump(b, STRIP_OUT.from, 0.25) - (STRIP_PANE.y + STRIP_PANE.h + 40) * swiftIn(progress(STRIP_OUT.from + 0.125, STRIP_OUT.to, b));
  return { pane: { ...STRIP_PANE, y: STRIP_PANE.y + dy, baseline0: STRIP_PANE.baseline0 + dy }, scroll: 1 };
}

function drawPane(ctx: CanvasRenderingContext2D, b: number, t: number): void {
  const at = paneAt(b);
  if (!at) return;
  const lines = b < ROW_AT ? LINES_FILES : LINES_STASHED;
  const lay = drawTerm(ctx, at.pane, lines, { t, scroll: at.scroll });
  // The stash row lands with a cyan flash across it and a ring round its ✔.
  const f = progress(ROW_AT, ROW_AT + 1.5, b);
  if (b >= ROW_AT && f < 1) {
    const cell = lay.cell(2, 0);
    const x1 = lay.col(Array.from(lines[2]).length) + 12;
    const x0 = lay.col(0) - 12;
    ctx.save();
    roundedRect(ctx, at.pane.x, at.pane.y, at.pane.w, at.pane.h, 12);
    ctx.clip();
    ctx.globalCompositeOperation = "lighter";
    const decay = (1 - f) ** 2.2;
    ctx.fillStyle = rgba(PALETTE.cyan, 0.2 * decay);
    roundedRect(ctx, x0, cell.y + 2, x1 - x0, cell.h - 4, 8);
    ctx.fill();
    // A brighter edge running along the row on the downbeat.
    const s = swiftOut(progress(ROW_AT, ROW_AT + 0.375, b));
    const sx = lerp(x0, x1, s);
    const g = ctx.createLinearGradient(sx - 160, 0, sx, 0);
    g.addColorStop(0, rgba(PALETTE.cyanBright, 0));
    g.addColorStop(1, rgba(PALETTE.cyanBright, 0.35 * (1 - progress(ROW_AT + 0.25, ROW_AT + 0.5, b))));
    ctx.fillStyle = g;
    ctx.fillRect(Math.max(x0, sx - 160), cell.y + 2, Math.min(160, sx - x0), cell.h - 4);
    ctx.restore();
    ring(ctx, cell.x + cell.w / 2, cell.y + cell.h / 2 - 2, 34, progress(ROW_AT, ROW_AT + 0.75, b), PALETTE.green, 3);
  }
}

// The card.

/** The card collapsing: away gently, then fast, then a soft finish into the name. */
const SHRINK = cubicBezier(0.45, 0, 0.25, 1);

/** The card's pose: rising into place, then collapsing into its lane label about the flying name. */
interface CardPose {
  dy: number;
  rot: number;
  /** Scale about the tab name's left end on its baseline, which is where `name` puts it. */
  scale: number;
  name: { x: number; y: number; size: number; color: string };
  alpha: number;
}

function cardPose(b: number): CardPose | null {
  if (b < CARD_IN.from || b >= CARD_OUT.to) return null;
  // Launched from below the frame, it reaches its place on b1.5, rises a hair past it and settles.
  const up = cardRise(b);
  const rest = { x: TAB_AT.x, y: TAB_AT.y, size: TAB_AT.size, color: PALETTE.text2 };
  if (b < CARD_OUT.from) {
    // A breath before it goes.
    const swell = 1 + 0.018 * bump(b, CARD_OUT.from - 0.375, 0.375);
    // It gives a little as the strip tears free of it.
    const recoil = -3 * bump(b, PEEL.to, 0.3);
    return { dy: (1 - up) * (CARD_FROM_Y - CARD.y) + recoil, rot: -3.2 * DEG * (1 - up), scale: swell, name: rest, alpha: 1 };
  }
  const k = progress(CARD_OUT.from, CARD_OUT.to, b);
  const e = swiftInOut(k);
  return {
    dy: 0,
    rot: 0,
    scale: lerp(1, 0.06, SHRINK(progress(0, 0.8, k))),
    name: {
      x: lerp(TAB_AT.x, LABEL_AT.x, e),
      y: lerp(TAB_AT.y, LABEL_AT.y, e) - 46 * Math.sin(Math.PI * e),
      size: lerp(TAB_AT.size, LABEL_AT.size, e),
      color: mix(PALETTE.text2, PALETTE.text1, e),
    },
    alpha: 1 - smoothstep(0.25, 0.7, k),
  };
}

/** Where the read band's leading edge is at beat b. */
const readEdge = (b: number): number => lerp(L.rowTop(0) - 20, L.rowBottom(9) + 30, inOutSine(progress(READ.from, READ.to, b)));

/** A lint squiggle from x0 to x1 under baseline y, drawn on left to right to p. */
function squiggle(ctx: CanvasRenderingContext2D, x0: number, x1: number, y: number, p: number): void {
  if (p <= 0) return;
  const end = lerp(x0, x1, p);
  ctx.save();
  ctx.strokeStyle = PALETTE.warm;
  ctx.lineWidth = 2.4;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  for (let x = x0; x <= end + 0.01; x += 1.5) {
    const yy = y + 2.2 * Math.sin((TAU * (x - x0)) / 10);
    if (x === x0) ctx.moveTo(x, yy);
    else ctx.lineTo(x, yy);
  }
  ctx.stroke();
  ctx.restore();
  if (p < 1) glow(ctx, end, y, 22, PALETTE.warm, 0.55 * (1 - p));
}

/** The card's rise, float and recoil: a shift and a turn about its middle. */
function cardTransform(ctx: CanvasRenderingContext2D, pose: CardPose): void {
  if (!pose.dy && !pose.rot) return;
  const cx = CARD.x + CARD.w / 2;
  const cy = CARD.y + CARD.h / 2;
  ctx.translate(cx, cy + pose.dy);
  ctx.rotate(pose.rot);
  ctx.translate(-cx, -cy);
}

function drawMainCard(ctx: CanvasRenderingContext2D, b: number): void {
  const pose = cardPose(b);
  if (!pose) return;
  // The name on the tab, flying to its lane as the card collapses into it.
  const drawName = () => {
    if (b >= CARD_OUT.from) drawMono(ctx, TAB, pose.name.x, pose.name.y, pose.name.size, pose.name.color);
  };
  if (pose.alpha <= 0) {
    drawName();
    return;
  }
  ctx.save();
  ctx.globalAlpha *= pose.alpha;
  cardTransform(ctx, pose);
  if (pose.scale !== 1) {
    // About the name's left end on its baseline, so the card shrinks into
    // the name as it flies and the name never hangs off the card's left.
    ctx.translate(pose.name.x, pose.name.y);
    ctx.scale(pose.scale, pose.scale);
    ctx.translate(-TAB_AT.x, -TAB_AT.y);
  }
  // A soft shadow under it: straight down whatever the card's angle, and
  // clear of the captions' band.
  const d = Math.hypot(ctx.getTransform().a, ctx.getTransform().b);
  const breath = holdEnv(b) * Math.sin(Math.PI * (b - HOLD.from));
  ctx.save();
  ctx.shadowColor = `rgba(0,0,0,${(0.5 + 0.08 * breath).toFixed(4)})`;
  ctx.shadowBlur = (22 + 3 * breath) * d;
  ctx.shadowOffsetY = (10 + 2 * breath) * d;
  roundedRect(ctx, CARD.x, CARD.y, CARD.w, CARD.h, 16);
  ctx.fillStyle = PALETTE.surface;
  ctx.fill();
  ctx.restore();

  // Collapsing, the card's tab strip fades out first, so the name, growing
  // toward its lane size, never overhangs a tab smaller than itself.
  const tabA = b < CARD_OUT.from ? 1 : 1 - smoothstep(0, 0.2, progress(CARD_OUT.from, CARD_OUT.to, b));
  drawCard(ctx, CARD, { tab: tabA >= 1 ? TAB_BLANK : undefined, lines: MAIN_PY_STAGED, ...CARD_TEXT, gutter: { color: PALETTE.cyanSoft, from: 0, to: 9 } });
  if (tabA > 0 && tabA < 1) {
    ctx.save();
    ctx.beginPath();
    ctx.rect(CARD.x, CARD.y, CARD.w, TAB_BAND);
    ctx.clip();
    ctx.globalAlpha *= tabA;
    drawCard(ctx, CARD, { tab: TAB_BLANK, lines: [], ...CARD_TEXT, frame: false });
    ctx.restore();
  }
  if (b < CARD_OUT.from) drawMono(ctx, TAB, TAB_AT.x, TAB_AT.y, TAB_AT.size, PALETTE.text2);

  ctx.save();
  roundedRect(ctx, CARD.x, CARD.y, CARD.w, CARD.h, 16);
  ctx.clip();
  // Where the line was cut from: a recess with the cut marks round it, healing over.
  if (b >= PEEL.from) {
    const heal = 1 - smoothstep(3.25, 4.5, b);
    if (heal > 0) {
      ctx.save();
      ctx.globalAlpha *= heal;
      roundedRect(ctx, STRIP.x, STRIP.y, STRIP.w, STRIP.h, STRIP.r);
      ctx.fillStyle = "rgba(2,6,10,0.34)";
      ctx.fill();
      roundedRect(ctx, STRIP.x + 1, STRIP.y + 1, STRIP.w - 2, STRIP.h - 2, STRIP.r - 1);
      ctx.setLineDash([...DASH]);
      ctx.strokeStyle = rgba(PALETTE.paper, 0.3);
      ctx.lineWidth = 2;
      ctx.stroke();
      ctx.restore();
    }
  } else {
    drawStripFace(ctx, STRIP.x, STRIP.y, { dash: dashAt(b) });
  }
  // The staged lines light up in the gutter as the read band passes them, and stay lit.
  if (b >= READ.from) {
    const edge = readEdge(b);
    const pulse = 1 + 0.1 * holdPulse(b);
    for (let r = 0; r <= 9; r++) {
      const lit = smoothstep(L.rowTop(r), L.rowBottom(r), edge);
      if (lit <= 0) continue;
      ctx.fillStyle = rgba(PALETTE.cyan, 0.8 * lit * pulse);
      ctx.fillRect(CARD.x + 12, L.rowTop(r), 6, L.rowBottom(r) - L.rowTop(r));
    }
    const env = smoothstep(READ.from, READ.from + 0.25, b) * (1 - smoothstep(READ.to - 0.25, READ.to, b));
    if (env > 0) {
      ctx.save();
      ctx.beginPath();
      ctx.rect(CARD.x, L.rowTop(0) - 6, CARD.w, L.rowBottom(9) - L.rowTop(0) + 8);
      ctx.clip();
      const bandH = 96;
      const g = ctx.createLinearGradient(0, edge - bandH, 0, edge);
      g.addColorStop(0, rgba(PALETTE.cyan, 0));
      g.addColorStop(0.8, rgba(PALETTE.cyan, 0.16 * env));
      g.addColorStop(1, rgba(PALETTE.cyan, 0.28 * env));
      ctx.fillStyle = g;
      ctx.fillRect(CARD.x, edge - bandH, CARD.w, bandH);
      ctx.fillStyle = rgba(PALETTE.cyanBright, 0.8 * env);
      ctx.fillRect(CARD.x + 12, edge - 1, CARD.w - 24, 2);
      glow(ctx, CARD.x + CARD.w / 2, edge, 60, PALETTE.cyan, 0.12 * env);
      ctx.restore();
    }
  }
  ctx.save();
  // Full on the beats, a touch dimmer between them.
  ctx.globalAlpha *= 1 - 0.1 * (holdEnv(b) - holdPulse(b));
  for (const q of SQUIGGLES) {
    const p = swiftOut(progress(q.at, q.at + SQUIGGLE_DRAW, b));
    squiggle(ctx, L.col(q.col0), L.col(q.col1) - 2, L.baseline(q.row) + 6.5, p);
  }
  ctx.restore();
  ctx.restore();
  ctx.restore();
  drawName();
}

/** The `unstaged` tag on line 11, with a dashed leader to it, riding with the card; the sparks blow it away. */
function drawTag(ctx: CanvasRenderingContext2D, b: number): void {
  const inP = swiftOut(progress(TAG_IN, TAG_IN + 0.375, b));
  const blow = progress(SPARKS_AT, SPARKS_AT + 0.22, b);
  const pose = cardPose(b);
  if (inP <= 0 || blow >= 1 || !pose) return;
  ctx.save();
  cardTransform(ctx, pose);
  const leader = inP * (1 - progress(1.9375, 2.0625, b));
  if (leader > 0) {
    ctx.save();
    ctx.globalAlpha *= 0.7 * leader;
    ctx.strokeStyle = PALETTE.text3;
    ctx.lineWidth = 2;
    ctx.setLineDash([6, 6]);
    ctx.beginPath();
    const y = STRIP.y + STRIP.h / 2;
    ctx.moveTo(STRIP.x + STRIP.w + 10, y);
    ctx.lineTo(lerp(STRIP.x + STRIP.w + 10, TAG.x - 14, inP), y);
    ctx.stroke();
    ctx.restore();
  }
  // Blown along by the sparks: away to the right, fading.
  const k = swiftOut(blow);
  ctx.save();
  ctx.globalAlpha *= inP * (1 - blow) ** 1.5;
  drawText(ctx, "unstaged", TAG.x - 16 * (1 - inP) + 22 * k, TAG.y - 8 * k, { font: font(40, 500), fill: PALETTE.text3 });
  ctx.restore();
  ctx.restore();
}

// The tray.

function lidAt(b: number): number {
  if (b < LID.open) return 0;
  if (b < LID.shut) return land(b, LID.open, LID.opened - LID.open, 0.25) + 0.05 * Math.sin(TAU * 2 * (b - LID.opened)) * (1 - progress(LID.opened, LID.opened + 0.5, b)) * (b > LID.opened ? 1 : 0);
  if (b < LID.slam) return 1 - progress(LID.shut, LID.slam, b) ** 3;
  // It bounces once off the rim.
  return 0.07 * bump(b, LID.slam, 0.3);
}

/** Paper dust puffed out of the slot as the lid slams: seeded motes, rising and fading. */
const DUST = Array.from({ length: 10 }, (_, i) => ({
  x: TRAY.sliver.x0 + 8 + (TRAY.sliver.x1 - TRAY.sliver.x0 - 16) * hash(i, 41),
  rise: 18 + 30 * hash(i, 42),
  drift: (hash(i, 43) - 0.5) * 36,
  r: 1.4 + 1.6 * hash(i, 44),
  life: 0.5 + 0.4 * hash(i, 45),
}));

function drawTrayLayer(ctx: CanvasRenderingContext2D, b: number, t: number): void {
  if (b < TRAY_IN.from) return;
  const up = springTo(b, TRAY_IN.from, TRAY_IN.to, 2.4, 0.66);
  const dy = (1 - up) * 120 + 3 * bump(b, LID.slam, 0.2);
  const alpha = smoothstep(TRAY_IN.from, TRAY_IN.from + 0.3, b);
  const pose = stripPose(b);
  // The strip comes down over the open lid's hinge end, so it flies in front
  // of the lid, and goes under it once it is flat in the slot, as the lid
  // comes down on it.
  const under = pose !== null && b >= INTO.from;
  // The line's top edge is the sliver in the slot once it is down in the tray.
  const holding = progress(INTO.from, INTO.to, b);
  ctx.save();
  if (dy) ctx.translate(0, dy);
  drawTray(ctx, {
    lid: lidAt(b),
    holding: holding >= 1 ? true : holding,
    t,
    alpha,
    // In the slot, the strip is in the tray's inside layer, under the lid,
    // in frame coordinates.
    inside: under
      ? (c) => {
          c.translate(0, -dy);
          drawStrip(c, pose);
        }
      : undefined,
  });
  // The slam puffs a little light and paper dust out of the slot.
  const since = b - LID.slam;
  if (since >= 0 && since < 1) {
    const cx = (TRAY.sliver.x0 + TRAY.sliver.x1) / 2;
    const cy = (TRAY.sliver.y0 + TRAY.sliver.y1) / 2;
    glow(ctx, cx, cy, 110, PALETTE.paper, 0.35 * (1 - progress(0, 0.75, since)) ** 2);
    ctx.save();
    for (const m of DUST) {
      const k = since / m.life;
      if (k >= 1) continue;
      const e = outCubic(k);
      ctx.fillStyle = rgba(PALETTE.paper, 0.55 * (1 - k) ** 1.5);
      ctx.beginPath();
      ctx.arc(m.x + m.drift * e, TRAY.sliver.y0 - 2 - m.rise * e, m.r * (1 - 0.4 * k), 0, TAU);
      ctx.fill();
    }
    ctx.restore();
  }
  ctx.restore();
  if (pose && !under) drawStrip(ctx, pose);
}

// The lanes coming up.

function drawLanesIn(ctx: CanvasRenderingContext2D, b: number): void {
  const order = [0, 1, -1, 2];
  const chars = LANE_FILES.map((f, i) => (order[i] < 0 ? 0 : Math.min(f.length, Math.floor((b - (TYPE.from + order[i] * TYPE.each)) * TYPE.rate + 1e-9))));
  drawLanes(ctx, {
    labels: LANE_FILES.map((_, i) => (i === 2 ? (b >= CARD_OUT.to ? 1 : 0) : 1)),
    chars: chars.map((c, i) => (i === 2 ? Infinity : Math.max(0, c))),
    tracks: LANES.rows.map((_, i) => swiftOut(progress(TRACKS.from + i * TRACKS.each, TRACKS.from + i * TRACKS.each + TRACKS.dur, b))),
    locks: 0,
  });
  // A caret after each label while it types.
  LANE_FILES.forEach((f, i) => {
    if (order[i] < 0) return;
    const c = chars[i];
    if (c <= 0 || b > TYPE.from + order[i] * TYPE.each + f.length / TYPE.rate + 0.125) return;
    ctx.fillStyle = rgba(PALETTE.cyan, 0.9);
    ctx.fillRect(LANES.labelX + monoWidth(f.slice(0, c), 40) + 3, LANES.rows[i] - 18, 3, 40);
  });
  LANES.rows.forEach((cy, i) => {
    const at = LOCKS.from + i * LOCKS.each;
    const k = land(b, at, LOCKS.dur, 0.3);
    if (k <= 0) return;
    drawPadlock(ctx, LANES.lockX, cy, "open", { scale: lerp(0.4, 1, k), alpha: clamp(progress(at, at + LOCKS.dur * 0.5, b)) });
  });
}

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    const b = lt / BEAT;
    if (b >= SETTLED) {
      drawHandoff(ctx, "stash|lanes", env);
      return;
    }
    bg(ctx, env);
    if (b >= CARD_OUT.from) drawLanesIn(ctx, b);
    drawPane(ctx, b, env.t);
    drawMainCard(ctx, b);
    drawTag(ctx, b);
    drawTrayLayer(ctx, b, env.t);
    drawBlade(ctx, b);
    drawSparks(ctx, b);
  },
  lit(lt) {
    const b = lt / BEAT;
    const at = paneAt(b);
    if (!at) return null;
    const a = 1 - smoothstep(STRIP_OUT.from + 0.25, STRIP_OUT.to, b);
    return termLit(at.pane, a);
  },
  captions: () => CAPTIONS,
};
