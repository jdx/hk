// Scene 2, "Steps in hk.pkl" (storyboard §6.2). The wordmark recedes and
// hk's builtin names wash in as three rivers at three depths (config-rivers.ts).
// `prettier` and `zizmor` pop out of them under "Builtins, from prettier to
// zizmor.", then zizmor sinks back and the rivers swing into the left third
// as the hk.pkl card draws itself on the right. The seven steps the commit
// will run are plucked out of the rivers, one per sixteenth: each name
// swings out of its column, turning upright, to its own row's height left
// of the card's text, glides along that still-empty row onto its
// `Builtins.` reference, and its line unfolds round it; the rivers make way
// round each name as it pulls out. The closing brace stamps the block shut,
// the rivers ebb, a faint sheen glances across the resting card, and under
// "Configured in Pkl: typed and reusable." a warm band marks ruff-format, a
// builtin amended with `depends`, its mark breathing. Then the card folds
// into the hk.pkl chip, its tab's name becoming the chip's, for `commit`.
//
// Every frame is a pure function of the scene's local time. The first frame
// is open|config's wordmark and everything from b11.75 is config|commit's
// chip, drawn by the handoffs themselves.

import { BAR, BEAT, PALETTE, type Scene, sec } from "../bible";
import { mix, rgba } from "../color";
import { glow, makeCanvas, roundedRect } from "../fx";
import { drawHandoff } from "../handoff";
import {
  CARD_TAB,
  type CardLine,
  cardLayout,
  drawCard,
  drawMono,
  HKPKL_CHIP,
  HKPKL_LINES,
  HKPKL_STEPS,
  pklRuns,
} from "../kit/card";
import { drawLogo, LOGO_FULL, LOGO_OPEN } from "../kit/logo";
import { jolt, lerpRect, type Pt, type Rect } from "../kit/motion";
import {
  clamp,
  cubicBezier,
  hash,
  inOutCubic,
  inQuad,
  lerp,
  outCubic,
  outSine,
  progress,
  smoothstep,
  spring,
  swiftOut,
  TAU,
} from "../math";
import { type Caption, drawText, font, MONO } from "../type";
import {
  BACK_BLUR,
  drawName,
  drawRivers,
  type Fixed,
  layoutRivers,
  maskAt,
  namePose,
  type Pose,
  RIVERS,
  type RiverName,
  riverPose,
} from "./config-rivers";

const S = sec("config");
/** Local seconds of section beat `n`. */
const b = (n: number): number => n * BEAT;

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 5 words: need 3.5, hold 4.
  { out: 6.5, lines: [{ in: 2.5, text: "Builtins, from `prettier` to `zizmor`." }] },
  // 3 + 3 words: need 4, hold 4 (line 2 needs 2.5, holds 3.5).
  { out: 11.5, lines: [{ in: 7.5, text: "Configured in Pkl:" }, { in: 8, text: "typed and reusable." }] },
];

// Beat map, local seconds. The score (score/config.ts) can be written to these.
/** The wordmark recedes over b0–1. */
export const T_LOGO_OUT = b(1);
/** `prettier` and `zizmor` pop out of the rivers, each with a cyan ring. */
export const T_POP = b(2);
/**
 * zizmor sinks back into its river. It is back to a river name's size by
 * 60% of the way and back in its file by about 80% (b3.75): the plop.
 */
export const T_SINK = [b(3.25), b(3.875)] as const;
/** The card stroke-draws, outline then fill: the paper flick. */
export const T_CARD = [b(4), b(4.5)] as const;
/**
 * The seven steps land one per sixteenth, b4.5 to b6 (storyboard: "b4.5–6.25,
 * one per sixteenth"), each after a half-beat flight: the fiddle's plucks go
 * on the landings.
 */
export const T_LAND = HKPKL_STEPS.map((_, i) => b(4.5 + i / 4));
export const T_TAKEOFF = T_LAND.map((t) => t - b(0.5));
/** The closing brace stamps in. */
export const T_STAMP = b(6.5);
/** The rivers ebb. */
const T_EBB = [b(6), b(7)] as const;
/** The warm band sweeps ruff-format's three lines: a builtin, amended. */
export const T_BAND = b(8.5);
/** The card folds into the hk.pkl chip. */
export const T_FOLD = [b(10.5), b(11.75)] as const;

// The card.

const CARD: Rect = { x: 560, y: 110, w: 1240, h: 590 };
const CARD_OPTS = { size: 34, lineH: 44, lang: "pkl" as const, textX: 600 };
const L = cardLayout(CARD, CARD_OPTS);
const ADV = L.advance;
const TEXT = HKPKL_LINES.map((l) => (typeof l === "string" ? l : ""));

/** Each plucked step: its builtin, its row, and the column its `Builtins.` name starts in. */
const STEPS = HKPKL_STEPS.map((st, i) => {
  const text = TEXT[st.line];
  const col = text.indexOf(`Builtins.${st.builtin}`) + "Builtins.".length;
  return { ...st, i, text, col, takeoff: T_TAKEOFF[i], land: T_LAND[i] };
});
type Step = (typeof STEPS)[number];

/** Where a landed name sits: its visual centre (drawName's origin) at 34 px. */
const landPose = (st: Step): Pose => ({
  x: L.col(st.col) + (st.builtin.length * ADV) / 2,
  y: L.baseline(st.line) - 0.34 * CARD_OPTS.size,
  a: 0,
});

// The rivers, with the names the scene handles itself placed where it needs
// them: the two pops crossing y 430 at x 420 and 1500 on the beat they pop,
// and each step (after prettier) in a column at a chosen height the moment
// it is plucked. The places are chosen so that no name, as it is picked,
// is in the way of the one flying before it: a name turning upright sweeps
// its start up and left and its end down and right, so the next is picked
// above and to the right of it, or clear to its left (checked by sampling
// every flight against every pick and drawn line, 1/960 s apart).

/** Popped words: centres and river placement. */
const POPS = [
  { text: "prettier", x: 420, y: 430 },
  { text: "zizmor", x: 1500, y: 430 },
] as const;
const POP_SIZE = 56;

/** Where each later step is plucked from: its river, file and height in the column. */
const PLUCK_FROM: Record<string, { river: number; lane: number; y: number }> = {
  ruff: { river: 0, lane: 1, y: 520 },
  ruff_format: { river: 1, lane: 0, y: 290 },
  shfmt: { river: 2, lane: 1, y: 580 },
  shellcheck: { river: 0, lane: 0, y: 250 },
  trailing_whitespace: { river: 1, lane: 1, y: 480 },
  newlines: { river: 0, lane: 1, y: 320 },
};

/** The s along river `ri`, file `lane`, that is at height `y` at `t`: found by a scan and a refining one. */
function solveS(ri: number, lane: number, y: number, t: number): number {
  const r = RIVERS[ri];
  const n = r.lanes[lane];
  let best = 0;
  let err = Infinity;
  for (let s = -3000; s <= 5000; s += 4) {
    const e = Math.abs(riverPose(r, s, n, t).y - y);
    if (e < err) [best, err] = [s, e];
  }
  for (let s = best - 4; s <= best + 4; s += 0.05) {
    const e = Math.abs(riverPose(r, s, n, t).y - y);
    if (e < err) [best, err] = [s, e];
  }
  return best;
}

const FIXED: Fixed[] = [
  ...POPS.map((p) => ({ text: p.text, river: 2, lane: 1, s: p.x, at: T_POP })),
  ...STEPS.slice(1).map((st) => {
    const f = PLUCK_FROM[st.builtin];
    return { text: st.builtin, river: f.river, lane: f.lane, s: solveS(f.river, f.lane, f.y, st.takeoff), at: st.takeoff };
  }),
];
const NAMES = layoutRivers(FIXED);
const named = (text: string): RiverName => {
  const nm = NAMES.find((x) => x.text === text);
  if (!nm) throw new Error(`config: ${text} is not in the rivers`);
  return nm;
};
const PRETTIER = named("prettier");
const ZIZMOR = named("zizmor");
const PLUCKED = new Map(STEPS.map((st) => [named(st.builtin), st]));

// Motion.

/** A name somewhere between the rivers and its own life: a pose, a size, a colour and an opacity. */
interface Look {
  pose: Pose;
  size: number;
  color: string;
  alpha: number;
  /** 0..1 how far it stands off the page: its shadow. */
  lift: number;
  /** Out of focus, px: a name leaving the back river sharpens as it flies. */
  blur?: number;
}

/** Device pixels per logical px: shadows and filters are in device pixels. */
const devScale = (ctx: CanvasRenderingContext2D): number => {
  const m = ctx.getTransform();
  return Math.hypot(m.a, m.b) || 1;
};

const riverLook = (nm: RiverName, t: number): Look => {
  const { pose, alpha } = namePose(nm, t);
  return { pose, size: nm.size, color: nm.color, alpha: alpha * maskAt(pose.y) * ebb(t), lift: 0, blur: nm.river === 0 ? BACK_BLUR : 0 };
};

/** The rivers' opacity: all there until b6, gone by b7. */
const ebb = (t: number): number => 1 - smoothstep(T_EBB[0], T_EBB[1], t);

/** How far a popped word has come out of its river: a spring that overshoots a little. */
const popOf = (t: number): number => (t < T_POP ? 0 : spring(t - T_POP, 2.8, 0.52));
/** zizmor's pop, eased back to 0 as it sinks into its river again. */
const zizmorPop = (t: number): number => popOf(t) * (1 - inOutCubic(progress(T_SINK[0], T_SINK[1], t)));
/**
 * zizmor's size as it sinks, 1 popped to 0 a river name's: it shrinks ahead
 * of its way back, so it is river-sized before it is among the names again.
 */
const zizmorSize = (t: number): number => popOf(t) * (1 - outCubic(progress(T_SINK[0], lerp(T_SINK[0], T_SINK[1], 0.6), t)));

/** A popped word's own place: its centre, bobbing a little on a bar-long sine, opposite for the two. */
function popHome(i: number, t: number): Pose {
  const p = POPS[i];
  const settle = smoothstep(T_POP, T_POP + 0.35, t);
  const bob = 4 * settle * Math.sin((TAU * (t - T_POP)) / BAR) * (i ? -1 : 1);
  return { x: p.x, y: p.y + bob, a: 0 };
}

function popLook(i: number, t: number): Look {
  const nm = i ? ZIZMOR : PRETTIER;
  const k = i ? zizmorPop(t) : popOf(t);
  const from = riverLook(nm, t);
  const home = popHome(i, t);
  const c = clamp(k);
  return {
    pose: { x: lerp(from.pose.x, home.x, k), y: lerp(from.pose.y, home.y, k), a: lerp(from.pose.a, 0, k) },
    size: lerp(from.size, POP_SIZE, i ? zizmorSize(t) : k),
    color: mix(from.color, PALETTE.warm, c),
    alpha: lerp(from.alpha, 1, c),
    lift: c,
  };
}

/** A step is picked out in its river a sixteenth before it is plucked. */
const PICK = b(0.25);

/**
 * A step in its river as it is picked: still flowing with it, it warms,
 * brightens and swells a little, and comes into focus.
 */
function pickLook(st: Step, t: number): Look {
  const k = riverLook(named(st.builtin), t);
  const p = smoothstep(st.takeoff - PICK, st.takeoff, t);
  return {
    pose: k.pose,
    size: k.size * (1 + 0.12 * p),
    color: mix(k.color, PALETTE.warm, p),
    alpha: lerp(k.alpha, 1, p),
    lift: 0.4 * p,
    blur: (k.blur ?? 0) * (1 - p),
  };
}

/** Where a step takes off from: prettier from its pop, the rest from their rivers. */
const takeoffLook = (st: Step): Look => (st.i === 0 ? popLook(0, st.takeoff) : pickLook(st, st.takeoff));
const TAKEOFF_LOOK = STEPS.map(takeoffLook);

// Flights. Each name swings out of its river, turning upright, to the height
// of its own row while it is still left of the card's text (its gate), then
// glides right along that row onto its `Builtins.` reference. The rows fill
// from the top and a row is empty until its name lands, so a name in flight
// crosses no line already drawn.

/** The step rows' first glyphs are at column 2: a flying name keeps its right edge left of this until it is level with its row. */
const GATE_EDGE = L.col(2) - 12;
/** A name's size at its gate: its size in the river, but at most 4 px over the card's, all the swell it keeps along its row. */
const gateSize = (from: Look): number => clamp(from.size, CARD_OPTS.size, CARD_OPTS.size + 4);
/** The swell a short name gets in its swing out, px; a long name, sweeping far as it turns, swells less. */
const SWELL = 8;
const swellOf = (text: string, from: Look): number => Math.min(SWELL, 24 / (0.6 * text.length)) * clamp((52 - from.size) / 12);
/** The wind-up: a little crouch back, px, before the launch. */
const WIND = 12;
/** A flying name's far end stays right of this as it turns: the stage's side margin. */
const MARGIN_L = 168;

interface Path {
  p0: Pt;
  c1: Pt;
  c2: Pt;
  gate: Pt;
  /** Arc length of the swing to the gate, and of the whole flight. */
  len1: number;
  len: number;
  /** The swing's cumulative arc length at SAMPLES + 1 even steps of its parameter. */
  acc: Float64Array;
  /** The swing's direction at takeoff, unit: the wind-up goes the other way. */
  tan0: Pt;
}

const SAMPLES = 96;

const bez3 = (a: Pt, b1: Pt, b2: Pt, c: Pt, u: number): Pt => {
  const v = 1 - u;
  return {
    x: v * v * v * a.x + 3 * v * v * u * b1.x + 3 * v * u * u * b2.x + u * u * u * c.x,
    y: v * v * v * a.y + 3 * v * v * u * b1.y + 3 * v * u * u * b2.y + u * u * u * c.y,
  };
};

/** A step's flight path: a cubic swing from where it takes off to its gate, arriving level, then along its row. */
function flightPath(st: Step, from: Look): Path {
  const to = landPose(st);
  const hw = (st.builtin.length * 0.6 * gateSize(from)) / 2;
  const p0 = { x: from.pose.x, y: from.pose.y };
  const gate = { x: GATE_EDGE - hw, y: to.y };
  const dx = gate.x - p0.x;
  const dy = gate.y - p0.y;
  // Along the column first, the way a plucked name reads, then round into
  // the row, arriving level. A name too long to turn where it is (its far
  // end would swing out past the margin) heads right first, to find room.
  const bind = clamp((hw - (p0.x - MARGIN_L)) / 60);
  const c1 = { x: p0.x + lerp(0.12, 0.6, bind) * dx, y: p0.y + lerp(0.6, 0.1, bind) * dy };
  const c2 = { x: gate.x - lerp(Math.max(70, 0.6 * Math.abs(dx), 0.25 * Math.abs(dy)), 70, bind), y: gate.y };
  const acc = new Float64Array(SAMPLES + 1);
  let prev = p0;
  for (let i = 1; i <= SAMPLES; i++) {
    const q = bez3(p0, c1, c2, gate, i / SAMPLES);
    acc[i] = acc[i - 1] + Math.hypot(q.x - prev.x, q.y - prev.y);
    prev = q;
  }
  const len1 = acc[SAMPLES];
  const l0 = Math.hypot(c1.x - p0.x, c1.y - p0.y) || 1;
  return { p0, c1, c2, gate, len1, len: len1 + (to.x - gate.x), acc, tan0: { x: (c1.x - p0.x) / l0, y: (c1.y - p0.y) / l0 } };
}

/** The point `s` px along a flight. */
function pathAt(f: Path, s: number): Pt {
  if (s <= 0) return f.p0;
  if (s >= f.len1) return { x: f.gate.x + (s - f.len1), y: f.gate.y };
  let lo = 0;
  let hi = SAMPLES;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (f.acc[mid] <= s) lo = mid;
    else hi = mid;
  }
  const u = (lo + (s - f.acc[lo]) / (f.acc[hi] - f.acc[lo] || 1)) / SAMPLES;
  return bez3(f.p0, f.c1, f.c2, f.gate, u);
}

const PATHS = STEPS.map((st) => flightPath(st, TAKEOFF_LOOK[st.i]));

/** A flight's pace along its path: a gather, a launch, and a long soft landing. */
const RIDE = cubicBezier(0.45, 0, 0.2, 1);

/** A step's look from its takeoff to its landing. */
function flightLook(st: Step, t: number): Look {
  const from = TAKEOFF_LOOK[st.i];
  const f = PATHS[st.i];
  const p = progress(st.takeoff, st.land, t);
  const s = RIDE(p) * f.len;
  const at = pathAt(f, s);
  // How far through the swing, and along the row.
  const f1 = clamp(s / f.len1);
  const f2 = clamp((s - f.len1) / (f.len - f.len1));
  const wind = WIND * Math.sin(Math.PI * clamp(p / 0.24)) ** 2;
  // Turned upright the shortest way, and at its gate size, well before the gate.
  let a0 = from.pose.a % TAU;
  if (a0 > Math.PI) a0 -= TAU;
  if (a0 < -Math.PI) a0 += TAU;
  const up = smoothstep(0, 0.75, f1);
  const swell = swellOf(st.builtin, from) * Math.sin(Math.PI * clamp(f1 / 0.75));
  const gs = gateSize(from);
  const size = f1 < 1 ? lerp(from.size, gs, up) + swell : lerp(gs, CARD_OPTS.size, smoothstep(0, 1, f2));
  const x = at.x - f.tan0.x * wind;
  // A long name out of the left-hand column turns no faster than there is
  // room for: its far end, swinging left, stays inside the margin.
  const hw = (st.builtin.length * 0.6 * size) / 2;
  const hh = 0.5 * size;
  const room = x - MARGIN_L;
  const r = Math.hypot(hw, hh);
  const aMin = room >= r ? 0 : clamp(Math.atan2(hh, hw) + Math.acos(clamp(room / r, -1, 1)), 0, Math.abs(a0));
  const a = Math.sign(a0) * Math.max(Math.abs(a0) * (1 - up), aMin);
  const grow = Math.sin(Math.PI * p);
  return {
    pose: { x, y: at.y - f.tan0.y * wind, a },
    size,
    color: mix(from.color, PALETTE.warm, clamp(p * 3)),
    alpha: lerp(from.alpha, 1, clamp(p * 4)),
    lift: Math.max(from.lift * (1 - p), Math.min(1, 1.6 * grow)),
    blur: (from.blur ?? 0) * (1 - clamp(p * 3)),
  };
}

/** The motion blur's shutter: one 120 fps frame. */
const SHUTTER = 1 / 120;
/**
 * At most this far apart, px (closer than a stem is wide), so the smear
 * reads as a streak and never as a comb of copies, even on a paused frame.
 */
const SMEAR_STEP = 1.5;
/**
 * The most ghosts a smear draws: SMEAR_STEP apart at the flights' top speed,
 * about 120 px a shutter (a long name's ends sweep further as it turns).
 */
const SMEAR_MAX = 96;
/** The smear's total opacity, spread over its ghosts, faintest furthest back. */
const SMEAR_ALPHA = 0.5;
/** The smear's softening, px: its ghosts are blurred this much as one. */
const SMEAR_SOFT = 1;
/** The layer the smear's ghosts are drawn on. */
let smearLayer: HTMLCanvasElement | null = null;

/** A flying name's motion blur: ghosts over the last shutter's worth of its path, as many as its speed needs. */
function drawSmear(ctx: CanvasRenderingContext2D, st: Step, t: number, k: Look): void {
  const t0 = Math.max(st.takeoff, t - SHUTTER);
  const back = flightLook(st, t0);
  const dist = Math.hypot(k.pose.x - back.pose.x, k.pose.y - back.pose.y) + (Math.abs(k.pose.a - back.pose.a) * st.builtin.length * 0.6 * k.size) / 2;
  if (dist < SMEAR_STEP) return;
  const n = Math.min(SMEAR_MAX, Math.ceil(dist / SMEAR_STEP));
  const ghosts: Look[] = [];
  for (let g = n; g >= 1; g--) {
    const kg = g === n ? back : flightLook(st, t - ((t - t0) * g) / n);
    const w = (2 * (1 - g / (n + 1))) / n;
    ghosts.push({ ...kg, alpha: kg.alpha * SMEAR_ALPHA * w, lift: 0, blur: 0 });
  }
  // The ghosts go down on a layer of their own, blurred a pixel as one, so
  // their stems (each set on the whole-pixel grid) run together into a
  // streak: just the few device pixels round them.
  const m = ctx.getTransform();
  const px = devScale(ctx);
  const pad = SMEAR_SOFT * 4 * px;
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const gh of ghosts) {
    const r = Math.hypot((st.builtin.length * 0.6 * gh.size) / 2, 0.7 * gh.size) * px + pad;
    const dx = m.a * gh.pose.x + m.c * gh.pose.y + m.e;
    const dy = m.b * gh.pose.x + m.d * gh.pose.y + m.f;
    x0 = Math.min(x0, dx - r);
    y0 = Math.min(y0, dy - r);
    x1 = Math.max(x1, dx + r);
    y1 = Math.max(y1, dy + r);
  }
  const { width, height } = ctx.canvas;
  x0 = clamp(Math.floor(x0), 0, width);
  y0 = clamp(Math.floor(y0), 0, height);
  x1 = clamp(Math.ceil(x1), 0, width);
  y1 = clamp(Math.ceil(y1), 0, height);
  if (x1 <= x0 || y1 <= y0) return;
  if (!smearLayer || smearLayer.width !== width || smearLayer.height !== height) smearLayer = makeCanvas(width, height);
  const layer = smearLayer.getContext("2d")!;
  layer.setTransform(1, 0, 0, 1, 0, 0);
  layer.globalAlpha = 1;
  layer.clearRect(x0, y0, x1 - x0, y1 - y0);
  layer.setTransform(m);
  for (const gh of ghosts) drawLook(layer, st.builtin, gh);
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.filter = `blur(${(SMEAR_SOFT * px).toFixed(2)}px)`;
  ctx.drawImage(smearLayer, x0, y0, x1 - x0, y1 - y0, x0, y0, x1 - x0, y1 - y0);
  ctx.restore();
}

/** A name drawn at a look, with a soft shadow as far as it is lifted. */
function drawLook(ctx: CanvasRenderingContext2D, text: string, k: Look): void {
  if (!(k.alpha > 0)) return;
  const px = devScale(ctx);
  ctx.save();
  ctx.globalAlpha *= clamp(k.alpha);
  if (k.lift > 0) {
    ctx.shadowColor = rgba("#000000", 0.6 * k.lift);
    ctx.shadowBlur = 22 * k.lift * px;
    ctx.shadowOffsetY = 10 * k.lift * px;
  }
  if ((k.blur ?? 0) > 0.05) ctx.filter = `blur(${((k.blur ?? 0) * px).toFixed(2)}px)`;
  ctx.fillStyle = k.color;
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
  ctx.translate(k.pose.x, k.pose.y);
  if (k.pose.a) ctx.rotate(k.pose.a);
  drawName(ctx, text, k.size);
  ctx.restore();
}

// The card's lines.

/** A line's glyphs with their Pkl colours, by column. */
const GLYPHS = TEXT.map((line) => {
  const out: { ch: string; col: number; color: string }[] = [];
  let col = 0;
  for (const run of pklRuns(line)) {
    for (const ch of run.text) {
      if (ch !== " ") out.push({ ch, col, color: run.color });
      col++;
    }
  }
  return out;
});

/** How warm a landed name still glows, 1 on landing, cooling to text1. */
const warmth = (st: Step, t: number): number => (t < st.land ? 1 : Math.exp(-(t - st.land) / 0.24));
/** From here a step's line is drawn by drawCard, exactly as it will stay. */
const settled = (st: Step): number => st.land + 1.7;

/** How the line unfolds round a landed name: when each glyph starts, from the name outward. */
function unfoldAt(st: Step, col: number): number {
  const left = st.col;
  const right = st.text.length - st.col - st.builtin.length;
  const dt = Math.min(0.014, 0.15 / Math.max(left, right, 1));
  if (col < st.col) return st.land + (st.col - 1 - col) * dt;
  return st.land + (col - st.col - st.builtin.length) * dt;
}
const GLYPH_IN = 0.07;

/** Rows 5 and 6 (ruff-format's `depends` and brace) clip in with row 4. */
const CLIP_IN = [T_LAND[2] + 0.05, T_LAND[2] + 0.2] as const;
/** The stamp: the brace comes down from the viewer over a sixteenth. */
const STAMP_FALL = b(0.25);
const STAMP_DONE = T_STAMP + 0.4;

/** The card's rows as drawCard draws them at `t`: settled rows whole, the rest left to the overlay. */
function cardLines(t: number): CardLine[] {
  const reveal = cardReveal(t);
  return HKPKL_LINES.map((line, row) => {
    if (row === 0 || row === 1) {
      const k = smoothstep(row ? 0.62 : 0.5, row ? 1 : 0.9, reveal);
      const base = typeof line === "string" ? { text: line } : line;
      return k >= 1 ? line : { ...base, alpha: k, dy: 8 * (1 - outCubic(k)) };
    }
    const st = STEPS.find((s) => s.line === row);
    const done = st ? t >= settled(st) : row === 11 ? t >= STAMP_DONE : t >= CLIP_IN[1];
    return done ? line : { text: TEXT[row], alpha: 0 };
  });
}

const cardReveal = (t: number): number => outSine(progress(T_CARD[0], T_CARD[1], t));

/** A step's line while it is still landing and unfolding. */
function drawStepLine(ctx: CanvasRenderingContext2D, st: Step, t: number): void {
  if (t < st.land || t >= settled(st)) return;
  const y = L.baseline(st.line);
  // The rest of the line, from the name outward.
  for (const g of GLYPHS[st.line]) {
    if (g.col >= st.col && g.col < st.col + st.builtin.length) continue;
    const p = progress(unfoldAt(st, g.col), unfoldAt(st, g.col) + GLYPH_IN, t);
    if (p <= 0) continue;
    const side = g.col < st.col ? 1 : -1;
    const dx = side * 14 * (1 - swiftOut(p));
    ctx.save();
    ctx.globalAlpha *= p;
    drawMono(ctx, g.ch, L.col(g.col) + dx, y, CARD_OPTS.size, g.color);
    ctx.restore();
  }
  // The name, warm as it lands, cooling; squashed a little by the landing.
  const w0 = warmth(st, t);
  const color = w0 < 0.004 ? PALETTE.text1 : mix(PALETTE.text1, PALETTE.warm, w0);
  const j = jolt(t, st.land, 0.16, 6.5);
  if (j !== 0) {
    const w = st.builtin.length * ADV;
    const cx = L.col(st.col) + w / 2;
    ctx.save();
    ctx.translate(cx, y);
    ctx.scale(1 + 0.07 * j, 1 - 0.16 * j);
    drawMono(ctx, st.builtin, -w / 2, 0, CARD_OPTS.size, color);
    ctx.restore();
  } else {
    drawMono(ctx, st.builtin, L.col(st.col), y, CARD_OPTS.size, color);
  }
}

/** Rows 5 and 6, clipping in downward under row 4. */
function drawClipRows(ctx: CanvasRenderingContext2D, t: number): void {
  if (t < CLIP_IN[0] || t >= CLIP_IN[1]) return;
  const k = swiftOut(progress(CLIP_IN[0], CLIP_IN[1], t));
  const top = L.rowTop(5);
  ctx.save();
  ctx.beginPath();
  ctx.rect(CARD.x, top, CARD.w, (L.rowBottom(6) - top) * k);
  ctx.clip();
  for (const row of [5, 6]) {
    for (const g of GLYPHS[row]) drawMono(ctx, g.ch, L.col(g.col), L.baseline(row) - 10 * (1 - k), CARD_OPTS.size, g.color);
  }
  ctx.restore();
}

/** The closing brace: down from the viewer, a squash on the hit, and a puff of dust. */
function drawStamp(ctx: CanvasRenderingContext2D, t: number): void {
  if (t < T_STAMP - STAMP_FALL || t >= STAMP_DONE) return;
  const g = GLYPHS[11][0];
  const x = L.col(g.col);
  const y = L.baseline(11);
  const cx = x + ADV / 2;
  const cy = y - 0.34 * CARD_OPTS.size;
  if (t < T_STAMP) {
    const p = progress(T_STAMP - STAMP_FALL, T_STAMP, t);
    const s = lerp(3.2, 1, inQuad(p));
    const px = devScale(ctx);
    ctx.save();
    ctx.globalAlpha *= clamp(p * 2.5);
    ctx.shadowColor = rgba("#000000", 0.5 * (1 - p));
    ctx.shadowBlur = 24 * (1 - p) * px;
    ctx.shadowOffsetY = 18 * (1 - p) * px;
    ctx.translate(cx, cy);
    ctx.scale(s, s);
    ctx.translate(-cx, -cy);
    drawMono(ctx, g.ch, x, y, CARD_OPTS.size, g.color);
    ctx.restore();
    return;
  }
  const d = t - T_STAMP;
  const j = jolt(t, T_STAMP, 0.3, 5);
  // It flashes bright on the hit, back to its own colour in a few frames.
  const flash = Math.exp(-d / 0.018);
  ctx.save();
  ctx.translate(cx, y);
  ctx.scale(1 + 0.22 * j, 1 - 0.28 * j);
  drawMono(ctx, g.ch, -ADV / 2, 0, CARD_OPTS.size, flash < 0.004 ? g.color : mix(g.color, PALETTE.text1, flash));
  ctx.restore();
  ctx.save();
  roundedRect(ctx, CARD.x, CARD.y, CARD.w, CARD.h, 16);
  ctx.clip();
  // A shock line along the row, running out from the brace and fading.
  const shock = 0.2;
  if (d < shock) {
    const u = d / shock;
    const reach = 26 + 190 * outCubic(u);
    const a = 0.5 * (1 - u) ** 1.5 * clamp(d / 0.012);
    const gr = ctx.createLinearGradient(cx - reach, 0, cx + reach, 0);
    gr.addColorStop(0, rgba(PALETTE.text2, 0));
    gr.addColorStop(0.5, rgba(PALETTE.text2, a));
    gr.addColorStop(1, rgba(PALETTE.text2, 0));
    ctx.fillStyle = gr;
    ctx.fillRect(cx - reach, y + 3, 2 * reach, 1.5);
  }
  // Dust: fine seeded motes kicked up and out in a shallow fan, falling back.
  const life = 0.34;
  if (d < life) {
    const u = d / life;
    for (let i = 0; i < 18; i++) {
      const dir = i % 2 ? 1 : -1;
      const ang = (0.2 + 0.75 * hash(i, 67)) * dir;
      const v = 110 + 230 * hash(i, 61);
      const px = cx + dir * 6 + Math.sin(ang) * v * d * (1 - 0.45 * u);
      const py = y - 2 - Math.cos(ang) * 0.55 * v * d + 520 * d * d;
      ctx.fillStyle = rgba(PALETTE.text2, 0.4 * (1 - u) ** 1.6 * clamp(d / 0.02));
      ctx.beginPath();
      ctx.arc(px, py, (0.7 + 0.9 * hash(i, 71)) * (1 - 0.4 * u), 0, TAU);
      ctx.fill();
    }
  }
  ctx.restore();
}

/** The card's edge flares as the brace lands: the block is sealed. */
function drawSeal(ctx: CanvasRenderingContext2D, t: number): void {
  const d = t - T_STAMP;
  if (d < 0 || d > 0.6) return;
  const a = 0.55 * Math.exp(-d / 0.13) * clamp(d / 0.02);
  ctx.save();
  roundedRect(ctx, CARD.x, CARD.y, CARD.w, CARD.h, 16);
  ctx.strokeStyle = rgba(PALETTE.text2, a);
  ctx.lineWidth = 1.5;
  ctx.stroke();
  ctx.restore();
}

/**
 * Between the stamp and the band, while the code rests, a faint cool sheen
 * glances once across the card, left to right, under the code: the card
 * still moves, and no glyph does.
 */
const SHEEN = [b(7.05), T_BAND] as const;

function drawSheen(ctx: CanvasRenderingContext2D, t: number): void {
  const p = progress(SHEEN[0], SHEEN[1], t);
  if (p <= 0 || p >= 1) return;
  // A soft band leaning like `/`, from just off the card's left edge to
  // just off its right at a steady pace, so it is on the card throughout.
  const half = 200;
  const lean = 0.36;
  const ux = Math.cos(lean);
  const uy = Math.sin(lean);
  const reach = half / ux + (CARD.h / 2) * Math.tan(lean) + 8;
  const cx = lerp(CARD.x - reach, CARD.x + CARD.w + reach, p);
  const cy = CARD.y + CARD.h / 2;
  const g = ctx.createLinearGradient(cx - ux * half, cy - uy * half, cx + ux * half, cy + uy * half);
  const a = 0.035;
  g.addColorStop(0, rgba(PALETTE.glint, 0));
  g.addColorStop(0.3, rgba(PALETTE.glint, 0.35 * a));
  g.addColorStop(0.5, rgba(PALETTE.glint, a));
  g.addColorStop(0.7, rgba(PALETTE.glint, 0.35 * a));
  g.addColorStop(1, rgba(PALETTE.glint, 0));
  ctx.fillStyle = g;
  ctx.fillRect(CARD.x, CARD.y, CARD.w, CARD.h);
}

/** The warm band's sweep over rows 4–6, 0..1. */
const bandSweep = (t: number): number => swiftOut(progress(T_BAND, T_BAND + b(0.75), t));

/**
 * The warm mark's breath once the band is down: a glow that swells to b10
 * and ebbs into the fold, on a two-beat sine, so the card, its code at
 * rest, still has a pulse.
 */
const breathOf = (t: number): number => smoothstep(T_BAND + b(0.5), T_BAND + b(1), t) * (0.5 + 0.5 * Math.cos((TAU * (t - b(10))) / b(2)));

/** The band's leading edge: a bright warm line that burns out as the sweep settles; a warm mark stays at its left, breathing. */
function drawBandEdge(ctx: CanvasRenderingContext2D, t: number, alpha = 1): void {
  const k = bandSweep(t);
  if (k <= 0) return;
  const top = L.rowTop(4);
  const h = L.rowBottom(6) - top;
  const breath = breathOf(t);
  ctx.save();
  ctx.globalAlpha *= alpha;
  if (breath > 0) {
    // A soft warm wash round the mark, as tall as the band.
    const x = CARD.x + 14;
    const g = ctx.createLinearGradient(x - 2, 0, x + 64, 0);
    g.addColorStop(0, rgba(PALETTE.warm, 0.2 * breath));
    g.addColorStop(1, rgba(PALETTE.warm, 0));
    ctx.fillStyle = g;
    ctx.fillRect(x - 2, top, 66, h);
  }
  ctx.fillStyle = breath > 0 ? mix(PALETTE.warm, PALETTE.warmBright, 0.6 * breath) : PALETTE.warm;
  ctx.fillRect(CARD.x + 12, top, 4, h * smoothstep(0, 0.35, k));
  ctx.restore();
  if (k >= 1) return;
  const a = (1 - k) ** 0.7;
  const x = CARD.x + 12 + (CARD.w - 24) * k;
  ctx.save();
  glow(ctx, x, top + h / 2, 70, PALETTE.warm, 0.35 * a);
  ctx.fillStyle = rgba(PALETTE.warmBright, 0.8 * a);
  ctx.fillRect(x - 1, top, 2, h);
  ctx.restore();
}

/** A soft drop shadow under a box, never reaching into the captions' band. */
function dropShadow(ctx: CanvasRenderingContext2D, r: Rect, radius: number, alpha: number): void {
  if (!(alpha > 0)) return;
  ctx.save();
  ctx.beginPath();
  ctx.rect(0, 0, 1920, 736);
  ctx.clip();
  // The box is drawn off the frame; only its shadow lands.
  const off = 4000;
  const px = devScale(ctx);
  ctx.shadowColor = rgba("#000000", 0.5 * alpha);
  ctx.shadowBlur = 44 * px;
  ctx.shadowOffsetX = off * px;
  ctx.shadowOffsetY = 14 * px;
  roundedRect(ctx, r.x - off, r.y, r.w, r.h, radius);
  ctx.fillStyle = "#000000";
  ctx.fill();
  ctx.restore();
}

/** The card and everything on it at `t`, before it folds. */
function drawTheCard(ctx: CanvasRenderingContext2D, t: number): void {
  const reveal = cardReveal(t);
  if (reveal <= 0) return;
  const body = progress(0.45, 1, reveal);
  ctx.save();
  dropShadow(ctx, CARD, 16, body);
  drawCard(ctx, CARD, {
    ...CARD_OPTS,
    tab: "hk.pkl",
    lines: cardLines(t),
    reveal,
    highlight: { from: 4, to: 6, sweep: bandSweep(t) },
    under: (c) => drawSheen(c, t),
  });
  ctx.save();
  roundedRect(ctx, CARD.x, CARD.y, CARD.w, CARD.h, 16);
  ctx.clip();
  drawBandEdge(ctx, t);
  drawClipRows(ctx, t);
  for (const st of STEPS) drawStepLine(ctx, st, t);
  ctx.restore();
  drawStamp(ctx, t);
  drawSeal(ctx, t);
  ctx.restore();
}

// The fold into the chip.

const CHIP: Rect = HKPKL_CHIP;
/** The tab's label, where drawCard puts it: left x and baseline. */
const TAB_LABEL = { x: CARD.x + CARD_TAB.pad, y: CARD.y + CARD_TAB.h / 2 + 0.3 * CARD_TAB.size };

/** The fold's ease: weighty, in and out. */
const FOLD = cubicBezier(0.5, 0, 0.25, 1);

function drawFold(ctx: CanvasRenderingContext2D, t: number): void {
  const e = FOLD(progress(T_FOLD[0], T_FOLD[1], t));
  const R = lerpRect(CARD, CHIP, e);
  // The edge warms from the card's hairline to the chip's cyan as it lands.
  const c = smoothstep(0.35, 1, e);
  const radius = lerp(16, HKPKL_CHIP.r, e);
  dropShadow(ctx, R, radius, 1 - smoothstep(0.2, 0.8, e));
  ctx.save();
  roundedRect(ctx, R.x, R.y, R.w, R.h, radius);
  ctx.fillStyle = PALETTE.surface;
  ctx.fill();
  ctx.strokeStyle = mix(PALETTE.divider, PALETTE.cyan, c);
  ctx.lineWidth = lerp(1, 1.5, c);
  ctx.stroke();
  ctx.restore();

  // The code, scaled with the card from its top left and fading, inside it.
  const s = R.w / CARD.w;
  const content = 1 - smoothstep(0.1, 0.6, e);
  if (content > 0) {
    ctx.save();
    roundedRect(ctx, R.x, R.y, R.w, R.h, radius);
    ctx.clip();
    ctx.transform(s, 0, 0, s, R.x - CARD.x * s, R.y - CARD.y * s);
    drawCard(ctx, { ...CARD, h: R.h / s }, {
      ...CARD_OPTS,
      tab: "hk.pkl",
      lines: HKPKL_LINES,
      highlight: { from: 4, to: 6, sweep: 1 },
      frame: false,
      tabLabelAlpha: 0,
      alpha: content,
    });
    drawBandEdge(ctx, t, content);
    ctx.restore();
  }

  // The tab's name flies to where the chip's label will be and becomes it:
  // aimed at that place, not at the shrinking card's middle, so it keeps
  // to the top of the card in one clean curve.
  const k = e;
  const tabW = "hk.pkl".length * 0.6 * CARD_TAB.size;
  const from: Pt = { x: TAB_LABEL.x + tabW / 2, y: TAB_LABEL.y };
  const to: Pt = { x: CHIP.x + CHIP.w / 2, y: CHIP.y + CHIP.h / 2 + 12 };
  // It rides the shrinking tab at first, then leaves it for the chip's middle.
  const onTab: Pt = { x: R.x + (from.x - CARD.x) * s, y: R.y + (from.y - CARD.y) * s };
  const u = smoothstep(0.1, 1, k);
  const at = { x: lerp(onTab.x, to.x, u), y: lerp(onTab.y, to.y, u) };
  const size = lerp(CARD_TAB.size * s, 36, u);
  const color = mix(PALETTE.text2, PALETTE.text1, u);
  // Per glyph on the grid until the last stretch, where it hands over to the
  // chip's own label (drawText, as drawChip draws it) in the same place.
  const hand = smoothstep(0.86, 0.985, k);
  if (hand < 1) {
    ctx.save();
    ctx.globalAlpha *= 1 - hand;
    drawMono(ctx, "hk.pkl", at.x - (6 * 0.6 * size) / 2, at.y, size, color);
    ctx.restore();
  }
  if (hand > 0) {
    drawText(ctx, "hk.pkl", at.x, at.y, { font: font(36, 400, MONO), fill: rgba(PALETTE.text1, hand), align: "center" });
  }
}

// The logo's exit.

/** Leaving: a slow start, then away. */
const AWAY = cubicBezier(0.55, 0, 0.6, 1);

/**
 * The wordmark: a breath in, then away toward (960, 300), fading. It is
 * gone by b0.8, before the front river's wash reaches the middle, so no
 * name washes in under it.
 */
function drawLogoOut(ctx: CanvasRenderingContext2D, t: number): void {
  if (t >= T_LOGO_OUT) return;
  const breath = 0.03 * Math.sin(Math.PI * progress(0, b(0.4), t));
  const away = AWAY(progress(b(0.1), T_LOGO_OUT, t));
  const h = LOGO_OPEN.h * (1 + breath) * lerp(1, 0.3, away);
  const cy = lerp(LOGO_OPEN.cy, 300, away);
  const alpha = 1 - smoothstep(b(0.3), b(0.8), t);
  if (alpha <= 0) return;
  drawLogo(ctx, { cx: LOGO_OPEN.cx, cy, h }, LOGO_FULL, { alpha });
}

// The pops.

/** A capsule ring round a popped word, growing and fading. */
function popRing(ctx: CanvasRenderingContext2D, i: number, t: number): void {
  const p = progress(T_POP, T_POP + 0.5, t);
  if (p <= 0 || p >= 1) return;
  const home = popHome(i, t);
  const w = POPS[i].text.length * 0.6 * POP_SIZE;
  const grow = outCubic(p);
  const padX = 18 + 70 * grow;
  const padY = 10 + 40 * grow;
  const h = POP_SIZE + 2 * padY;
  // A three-frame attack, so it pulses on rather than switching on.
  const attack = smoothstep(0, 0.025, t - T_POP);
  ctx.save();
  ctx.strokeStyle = rgba(PALETTE.cyanBright, 0.9 * attack * (1 - p) ** 1.4);
  ctx.lineWidth = (3.5 * (1 - p) + 0.75) * attack;
  roundedRect(ctx, home.x - w / 2 - padX, home.y - h / 2, w + 2 * padX, h, h / 2);
  ctx.stroke();
  ctx.restore();
}

/**
 * A pool of the stage's own dark behind a popped word, so it stands clear
 * of the rivers it came out of: a soft capsule, wider than the word.
 */
function backdrop(ctx: CanvasRenderingContext2D, k: Look, text: string): void {
  const a = 0.8 * clamp(k.lift);
  if (a <= 0) return;
  const w = text.length * 0.6 * k.size + 120;
  const h = k.size * 2.2;
  ctx.save();
  ctx.translate(k.pose.x, k.pose.y);
  ctx.scale(w / 2, h / 2);
  const g = ctx.createRadialGradient(0, 0, 0, 0, 0, 1);
  g.addColorStop(0, rgba(PALETTE.bg, a));
  g.addColorStop(0.55, rgba(PALETTE.bg, a * 0.75));
  g.addColorStop(1, rgba(PALETTE.bg, 0));
  ctx.fillStyle = g;
  ctx.fillRect(-1, -1, 2, 2);
  ctx.restore();
}

/** prettier as it is at `t`: popped, then in flight. */
const prettierLook = (t: number): Look => (t < STEPS[0].takeoff ? popLook(0, t) : flightLook(STEPS[0], t));

/** The distance between segments ab and cd, px: 0 where they cross. */
function segmentGap(a: Pt, b1: Pt, c: Pt, d: Pt): number {
  const side = (p: Pt, q: Pt, r: Pt) => Math.sign((q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x));
  if (side(a, b1, c) * side(a, b1, d) < 0 && side(c, d, a) * side(c, d, b1) < 0) return 0;
  const toSeg = (p: Pt, q: Pt, r: Pt) => {
    const dx = r.x - q.x;
    const dy = r.y - q.y;
    const u = clamp(((p.x - q.x) * dx + (p.y - q.y) * dy) / (dx * dx + dy * dy || 1));
    return Math.hypot(p.x - q.x - u * dx, p.y - q.y - u * dy);
  };
  return Math.min(toSeg(a, c, d), toSeg(b1, c, d), toSeg(c, a, b1), toSeg(d, a, b1));
}

/** A name's centre line, end to end, at a pose. */
function spine(text: string, p: Pose, size: number): [Pt, Pt] {
  const h = (text.length * 0.6 * size) / 2;
  const c = Math.cos(p.a) * h;
  const s = Math.sin(p.a) * h;
  return [
    { x: p.x - c, y: p.y - s },
    { x: p.x + c, y: p.y + s },
  ];
}

/** How near, px between their centre lines, a river name is cleared round a plucked one. */
const PLUCK_CLEAR = 46;

/**
 * How much the rivers make way at pose `p` round the popped words (round
 * where each word is now, as wide as it is, until it has flown or sunk
 * back) and round each plucked name as it is picked and pulls out through
 * the columns: every name within reach of it fades back, and the river
 * closes behind it.
 */
function clearing(t: number): ((p: Pose, nm: RiverName) => number) | undefined {
  const f = [clamp(popOf(t)) * (1 - smoothstep(T_TAKEOFF[0], T_LAND[0], t)), Math.sqrt(clamp(zizmorPop(t)))];
  const at = [f[0] > 0 ? prettierLook(t) : null, f[1] > 0 ? popLook(1, t) : null];
  const carried = STEPS.filter((st) => st.i > 0 && t >= st.takeoff - PICK && t < st.land).map((st) => {
    const k = t < st.takeoff ? pickLook(st, t) : flightLook(st, t);
    return { line: spine(st.builtin, k.pose, k.size), f: 0.7 * smoothstep(st.takeoff - PICK, st.takeoff, t) };
  });
  // zizmor, sinking, crosses the names of both its river's files on its way
  // back to its own: they make way round it just as for a plucked name.
  const z = at[1];
  if (z && t > T_SINK[0]) carried.push({ line: spine("zizmor", z.pose, z.size), f: 0.85 * f[1] * smoothstep(T_SINK[0], T_SINK[0] + b(0.25), t) });
  if (f[0] <= 0 && f[1] <= 0 && !carried.length) return undefined;
  return (p, nm) => {
    let k = 1;
    at.forEach((w, i) => {
      if (!w) return;
      const rx = (POPS[i].text.length * 0.6 * w.size) / 2 + 160;
      const d = ((p.x - w.pose.x) / rx) ** 2 + ((p.y - w.pose.y) / 80) ** 2;
      k *= 1 - 0.85 * f[i] * Math.exp(-d * d);
    });
    if (carried.length) {
      const [a, b1] = spine(nm.text, p, nm.size);
      for (const q of carried) k *= 1 - q.f * Math.exp(-((segmentGap(a, b1, q.line[0], q.line[1]) / PLUCK_CLEAR) ** 4));
    }
    return k;
  };
}

/**
 * The scene's geometry, for tests and tooling (nothing draws from it): the
 * river names, the steps, the card's layout, and every name the scene is
 * carrying itself at `t`.
 */
export const PROBE = {
  names: NAMES,
  steps: STEPS,
  layout: L,
  paths: PATHS,
  clipIn: CLIP_IN,
  stampFrom: T_STAMP - STAMP_FALL,
  carried(t: number): { text: string; kind: "pop" | "pick" | "flight"; look: Look }[] {
    const out: { text: string; kind: "pop" | "pick" | "flight"; look: Look }[] = [];
    if (t >= T_POP && t < T_SINK[1]) out.push({ text: "zizmor", kind: "pop", look: popLook(1, t) });
    if (t >= T_POP && t < STEPS[0].takeoff) out.push({ text: "prettier", kind: "pop", look: popLook(0, t) });
    for (const st of STEPS) {
      if (st.i > 0 && t >= st.takeoff - PICK && t < st.takeoff) out.push({ text: st.builtin, kind: "pick", look: pickLook(st, t) });
      if (t >= st.takeoff && t < st.land) out.push({ text: st.builtin, kind: "flight", look: flightLook(st, t) });
    }
    return out;
  },
};

// The scene.

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    const t = lt;
    // From b11.75 the frame is config|commit's, drawn by the handoff itself.
    if (t >= T_FOLD[1]) {
      drawHandoff(ctx, "config|commit", env);
      return;
    }
    ctx.fillStyle = PALETTE.bg;
    ctx.fillRect(0, 0, env.W, env.H);
    if (t <= 0) {
      drawHandoff(ctx, "open|config", env);
      return;
    }
    drawLogoOut(ctx, t);

    // The rivers, less the names the scene is carrying itself.
    if (t >= b(0.5) && t < T_EBB[1]) {
      drawRivers(ctx, NAMES, t, {
        alpha: ebb(t),
        clear: clearing(t),
        skip: (nm) => {
          if (nm === PRETTIER) return t >= T_POP;
          if (nm === ZIZMOR) return t >= T_POP && t < T_SINK[1];
          const st = PLUCKED.get(nm);
          return st !== undefined && st.i > 0 && t >= st.takeoff - PICK;
        },
      });
    }

    // The popped words' pools of dark, over the rivers and under the card.
    // Sinking back, zizmor goes where its river goes, under the rivers' mask.
    const zizmor = t >= T_POP && t < T_SINK[1] ? popLook(1, t) : null;
    const z = zizmor && { ...zizmor, alpha: zizmor.alpha * maskAt(zizmor.pose.y) };
    if (z) backdrop(ctx, z, "zizmor");
    if (t >= T_POP && t < STEPS[0].land) {
      // prettier keeps its pool into its flight, letting it go as it launches.
      const k = prettierLook(t);
      const p = progress(STEPS[0].takeoff, STEPS[0].land, t);
      backdrop(ctx, { ...k, lift: (t < STEPS[0].takeoff ? k.lift : TAKEOFF_LOOK[0].lift) * (1 - smoothstep(0, 0.4, p)) }, "prettier");
    }

    if (t < T_FOLD[0]) drawTheCard(ctx, t);
    else drawFold(ctx, t);

    // The popped words, over everything.
    if (z) {
      popRing(ctx, 1, t);
      glow(ctx, z.pose.x, z.pose.y, 190, PALETTE.warm, 0.14 * z.lift * maskAt(z.pose.y));
      drawLook(ctx, "zizmor", z);
    }
    if (t >= T_POP && t < STEPS[0].takeoff) {
      const p = popLook(0, t);
      popRing(ctx, 0, t);
      glow(ctx, p.pose.x, p.pose.y, 210, PALETTE.warm, 0.14 * p.lift);
      drawLook(ctx, "prettier", p);
    }

    // The steps being picked out of their rivers.
    for (const st of STEPS) {
      if (st.i === 0 || t < st.takeoff - PICK || t >= st.takeoff) continue;
      drawLook(ctx, st.builtin, pickLook(st, t));
    }
    // The seven steps in flight, each smeared over a shutter's worth of its path.
    for (const st of STEPS) {
      if (t < st.takeoff || t >= st.land) continue;
      const k = flightLook(st, t);
      drawSmear(ctx, st, t, k);
      if (st.i === 0) glow(ctx, k.pose.x, k.pose.y, 210, PALETTE.warm, 0.14 * (1 - progress(st.takeoff, st.land, t)));
      drawLook(ctx, st.builtin, k);
    }
    // A warm puff where each lands.
    for (const st of STEPS) {
      const d = t - st.land;
      if (d < 0 || d > 0.4) continue;
      const lp = landPose(st);
      glow(ctx, lp.x, lp.y, 64 + 60 * d, PALETTE.warm, 0.3 * Math.exp(-d / 0.08) * smoothstep(0, 0.02, d));
    }
  },
  lit: () => null,
  captions: () => CAPTIONS,
};
