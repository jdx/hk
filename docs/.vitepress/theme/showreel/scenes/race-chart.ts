// The benchmark chart for `race` (storyboard §6.9): hk and every other tool
// the published run timed, one row each, racing on a shared clock. Every bar
// leaves the axis on the race's start beat and grows at the same speed, so
// the tips run flush, like a clock hand, until each tool's median is reached
// and its bar stops there: the stop beats are race-timing.ts's, the ones the
// score rings its dings on. Nothing is rounded or eased on the way, so a
// bar's length is always an honest duration, and at rest it is exactly
// median / axis × 1000 px. A stopped bar gets its median in the page's own
// format and its min–max whisker; the bracket then spans hk's end and the
// fastest rival's, and the caption says by how much.
//
// With two races the chart resets on b8.5: the bars reel back to the axis,
// the figures wipe, and the rows re-sort by the second race's medians on a
// spring (a tool that climbs two places jumps them) while the modes and the
// summary swap. While a finished chart holds, the camera leans in a little.
// The labels, figures and details wipe on b14.5, on a soft edge, and the
// bars ease into race|morph's four capsules by b15.75. With more than four
// tools the rows close up, without modes or whiskers (rowGeom), and the
// rows past the fourth fade as the others become the capsules.
//
// Everything here is a pure function of the section's local time and the
// facts; the model is cached by the facts object alone.

import { BEAT, PALETTE } from "../bible";
import { mix, rgba } from "../color";
import type { Race, ReelFacts } from "../facts";
import { glow, makeCanvas, roundedRect } from "../fx";
import { CAPSULES } from "../handoff";
import { type Curve, drawSpark, land, popIn } from "../kit/motion";
import { clamp, inOutCubic, inOutSine, keys, lerp, outQuart, progress, pulse, smoothstep, swiftInOut, swiftOut } from "../math";
import { type RaceRun, raceRuns } from "../race-timing";
import { drawText, font, layout, MONO } from "../type";

/** Local seconds of section beat `n`. */
const b = (n: number): number => n * BEAT;
/** One frame of the 120 fps render. */
const FRAME = 1 / 120;
/** The shutter for smears: one 60 fps frame of travel. */
const SHUTTER = 1 / 60;

// Layout (storyboard §6.9), logical px.

/** The axis: every bar grows from here. */
export const X0 = 700;
/** The axis's full scale, px: the scenario's largest max. */
export const SCALE = 1000;
export const BAR_H = CAPSULES.h;
const BAR_R = 8;
/** The capsule's end radius (race|morph). */
const CAP_R = CAPSULES.h / 2;
const NAME_X = 160;
const NAME_SIZE = 56;
/** Modes shrink to fit left of the axis, from 40 px to a 30 px floor; never abbreviated. */
const MODE_SIZE = 40;
const MODE_FLOOR = 30;
const MODE_MAX_W = X0 - NAME_X - 28;
/** The mode's baseline below its row's centre. */
const MODE_DY = 44;
const DETAIL_FONT = font(40, 500);
const DETAIL_MAX_W = 1600;
const MEDIAN_SIZE = 40;
/** The median label's gap from its bar's end, and the x it must not pass. */
const MEDIAN_GAP = 16;
const MEDIAN_LIMIT = 1740;
/** The tracks run a gap past the last x a median label may reach, so every label outside its bar sits on its track. */
const TRACK_W = MEDIAN_LIMIT + MEDIAN_GAP - X0;
/** The whisker runs under its bar, clear of the median label: its centre below the row's. */
const WHISKER_DY = BAR_H / 2 + 11;
const TICK = 16;
/** The bracket stands this far above hk's row. */
const BRACKET_DY = 40;
/** A bar waiting on the axis: a stub this wide. */
const NUB = 14;
/** Top details and the bottom one (storyboard §6.9). */
const WORKLOAD_AT = { x: 160, y: 150 } as const;
const SUMMARY_AT = { x: 160, y: 196 } as const;
/**
 * The bottom detail: the page has every scenario, the commit one included.
 * It sits under the bar column, ending on the right margin and starting no
 * further left than the axis, so it never reads as a third line under the
 * last row's name and mode; it shrinks to fit (40 px to a 30 px floor, like
 * the modes), and stays clear of the last whisker and the captions' band.
 */
export const FOOT = "All tools and scenarios, including commits: hk.jdx.dev/benchmarks";
export const FOOT_AT = { x: 1760, y: 716 } as const;
export const FOOT_MAX_W = FOOT_AT.x - X0;

// Cues, local seconds. The score (score/race.ts) is written to these and to
// raceRuns' stops.

/** Everything has whipped in and settled. */
export const SETTLED = b(1);
/** With two races, the first clears: bars reel back to the axis while the figures wipe. */
export const RESET = [b(8.5), b(8.75)] as const;
export const RESET_WIPE = [b(8.5), b(8.75)] as const;
/**
 * The rows re-sort by the second race's medians once the bars are home,
 * overshooting a little and landing on the second race's start.
 */
export const REORDER = [b(8.6875), b(9)] as const;
/** Before each start the waiting stubs crouch for a sixteenth: the anticipation. */
export const CROUCH = b(0.25);
/**
 * The bottom detail rises: on b12 with two races; with one, on b8, the
 * answer's downbeat, early in the long hold.
 */
export const FOOT_IN = b(12);
export const FOOT_IN_ONE = b(8);
/** Labels, figures, whiskers, bracket, details and tracks wipe left to right. */
export const EXIT_WIPE = [b(14.5), b(15)] as const;
/** The bars ease into the capsules, a 64th apart, all home by b15.75. */
export const MORPH = b(15);
export const MORPH_EACH = b(1 / 16);
export const MORPH_DUR = b(0.5);
/** From here the frame is race|morph. */
export const REST = b(15.75);
/**
 * While a finished chart holds, the camera leans in: 1.2% over the hold
 * about a point on the captions' top edge (so nothing grows toward them),
 * easing back to exactly 1 by the reset or the exit wipe. The hold starts
 * half a beat after the last bar stops.
 */
const HOLD_AFTER = b(0.5);
const HOLD_ZOOM = 0.012;
const HOLD_ANCHOR = { x: 960, y: 733 } as const;
const HOLD_BACK = b(1);
/** A glint crosses hk's bar a sixteenth after each race's last stop (the bracket), over GLINT_LEN seconds. */
export const GLINT_AFTER = b(0.25);
export const GLINT_LEN = 0.36;
/** With one race, a second glint crosses hk's bar in the middle of the long hold. */
export const HOLD_GLINT = b(6.5);
export const HOLD_GLINT_LEN = 0.4;
/**
 * With one race, as the hold's caption lands, a glint runs down every bar,
 * row by row from `from`, `each` apart, each `len` seconds long.
 */
export const ROLL_CALL = { from: b(9.5), each: b(0.125), len: 0.4 } as const;

// The model: each race's rows as the chart draws them.

export interface Entry {
  key: string;
  label: string;
  mode: string;
  /** The median as the page prints it. */
  shown: string;
  /** The bar at rest, px from the axis: median / axis × SCALE. */
  len: number;
  /** The whisker's ends, px from the axis: min and max. */
  lo: number;
  hi: number;
  /** Local seconds the bar stops (raceRuns). */
  stop: number;
  /** Px per second while it runs: the same for every bar in its race. */
  rate: number;
  /** Its row in this race: hk first, then the others by median. */
  row: number;
}

export interface RaceView {
  run: RaceRun;
  race: Race;
  /** Local seconds every bar leaves the axis. */
  start: number;
  /** Local seconds the slowest bar stops: the bracket draws from here. */
  last: number;
  entries: Readonly<Record<string, Entry>>;
  /** Subject keys by row. */
  order: readonly string[];
  /** The fastest other tool, whose bar end the bracket reaches. */
  rival: string;
}

export interface ChartModel {
  /** "6,157 files · 10 fixers · 8 CPUs". */
  workload: string;
  races: readonly RaceView[];
  /** Every subject, in the first race's row order, then any the second adds. */
  keys: readonly string[];
  /** Every row's metrics, for the most rows either race has. */
  geom: RowGeom;
}

/** A row's metrics, logical px. */
export interface RowGeom {
  barH: number;
  /** The name's and the median label's font sizes. */
  name: number;
  median: number;
  /** Whether the modes and the whiskers are drawn. */
  detail: boolean;
}

/**
 * The rows' metrics when a race has `n` rows. Up to four (the capsules'
 * rows) they are the storyboard's. With more, the rows close up, so the
 * bars, names and medians shrink to fit and the modes and whiskers are left
 * out (the page has them): no row's ink ever reaches the next row's.
 */
export function rowGeom(n: number): RowGeom {
  if (n <= CAPSULES.rows.length) return { barH: BAR_H, name: NAME_SIZE, median: MEDIAN_SIZE, detail: true };
  const pitch = slotY(1, n) - slotY(0, n);
  return {
    barH: Math.min(BAR_H, Math.round(0.6 * pitch)),
    name: Math.min(NAME_SIZE, Math.floor(0.7 * pitch)),
    median: Math.min(MEDIAN_SIZE, Math.floor(0.55 * pitch)),
    detail: false,
  };
}

/**
 * How far a row's ink reaches above and below its centre at `n` rows: its
 * name (baseline 2 px below the centre, ascenders up to 0.8 em, descenders
 * 0.25 em), its bar, its median label and, with detail, its whisker and mode.
 */
export function rowExtent(n: number): { above: number; below: number } {
  const g = rowGeom(n);
  const above = Math.max(g.barH / 2, 0.8 * g.name - 2);
  let below = Math.max(g.barH / 2, 2 + 0.25 * g.name, 0.6 * g.median);
  if (g.detail) below = Math.max(below, WHISKER_DY + TICK / 2, MODE_DY + 0.25 * MODE_SIZE);
  return { above, below };
}

function view(run: RaceRun): RaceView {
  const { race } = run;
  const start = b(run.start);
  const px = (s: number) => (s / race.axis) * SCALE;
  const entries: Record<string, Entry> = {};
  race.rows.forEach((r, row) => {
    const stop = b(run.stops[r.key]);
    const len = px(r.median);
    entries[r.key] = { key: r.key, label: r.label, mode: r.mode, shown: r.shown, len, lo: px(r.min), hi: px(r.max), stop, rate: len / (stop - start), row };
  });
  return {
    run,
    race,
    start,
    last: Math.max(...Object.values(entries).map((e) => e.stop)),
    entries,
    order: race.rows.map((r) => r.key),
    rival: race.claim.rival.key,
  };
}

const models = new Map<ReelFacts, ChartModel | null>();

/** The chart the facts back, or null when they back no race (F0 draws hk's terminal instead). */
export function chartModel(f: ReelFacts | null): ChartModel | null {
  if (!f) return null;
  const hit = models.get(f);
  if (hit !== undefined) return hit;
  if (models.size > 8) models.clear();
  const races = raceRuns(f).map(view);
  const keys = [...new Set(races.flatMap((r) => r.order))];
  const geom = rowGeom(Math.max(0, ...races.map((r) => r.order.length)));
  const m = races.length ? { workload: `${f.workload.files} files · ${f.workload.fixers} fixers · ${f.workload.cpus} CPUs`, races, keys, geom } : null;
  models.set(f, m);
  return m;
}

// Motion, all pure functions of local time.

/** A bar's length in race `r` at `lt`: the shared clock until its median, then exactly its median. */
function runLen(r: RaceView, key: string, lt: number): number {
  const e = r.entries[key];
  if (!e || lt <= r.start) return 0;
  return lt >= e.stop ? e.len : Math.min(e.len, e.rate * (lt - r.start));
}

/** The race whose bars, figures and modes are up at `lt`. */
export function raceAt(m: ChartModel, lt: number): RaceView {
  return m.races.length > 1 && lt >= RESET_WIPE[1] ? m.races[1] : m.races[0];
}

/** The race the chart ends on. */
const lastRace = (m: ChartModel): RaceView => m.races[m.races.length - 1];

/**
 * Subject `key`'s bar length at `lt`, px from the axis, before the exit:
 * the first race, then (with two) reeled back to the axis over the reset
 * and grown again in the second.
 */
export function barLen(m: ChartModel, key: string, lt: number): number {
  const [A, B] = m.races;
  if (!B || lt < RESET[0]) return runLen(A, key, lt);
  if (lt < B.start) return runLen(A, key, RESET[0]) * (1 - inOutCubic(progress(RESET[0], RESET[1], lt)));
  return runLen(B, key, lt);
}

/** How far the rows have re-sorted: easing out of rest, a 6% overshoot, home exactly on REORDER's end. */
const reorderK = keys([
  [REORDER[0], 0],
  [REORDER[0] + 0.72 * (REORDER[1] - REORDER[0]), 1.06, inOutCubic],
  [REORDER[1], 1, inOutSine],
]);

/** Row `i`'s centre when a race has `n` rows: the capsules' rows, or evenly over them for more. */
export const slotY = (i: number, n: number): number => {
  const rows = CAPSULES.rows;
  return n <= rows.length ? rows[i] : rows[0] + (i * (rows[rows.length - 1] - rows[0])) / (n - 1);
};

interface RowPos {
  y: number;
  /** 0..1 how far through a jump of two or more places. */
  jump: number;
  /** 0..1 how far through being passed by a jumping row: it ducks back. */
  duck: number;
  alpha: number;
  /** Its row in the first race, which sets its whip layer. */
  first: number;
}

/** Where subject `key`'s row stands at `lt`: re-sorting on a spring between the two races. */
export function rowPos(m: ChartModel, key: string, lt: number): RowPos {
  const [A, B] = m.races;
  const ea = A.entries[key];
  const first = ea ? ea.row : m.keys.indexOf(key);
  const ya = slotY(ea ? ea.row : first, A.order.length);
  if (!B) return { y: ya, jump: 0, duck: 0, alpha: ea ? 1 : 0, first };
  const eb = B.entries[key];
  const yb = eb ? slotY(eb.row, B.order.length) : ya;
  const k = reorderK(lt);
  const u = progress(REORDER[0], REORDER[1], lt);
  const places = ea && eb ? ea.row - eb.row : 0;
  // Only rows a jumper passes duck: those it overtakes.
  const passed = places < 0 && B.order.some((o) => {
    const a2 = A.entries[o];
    const b2 = B.entries[o];
    return !!a2 && !!b2 && a2.row - b2.row >= 2 && a2.row > ea!.row && b2.row < eb!.row;
  });
  const alpha = ea && eb ? 1 : ea ? 1 - progress(RESET[0], RESET[1], lt) : progress(REORDER[0], REORDER[1], lt);
  const arc = Math.sin(Math.PI * u);
  return { y: lerp(ya, yb, k), jump: places >= 2 ? arc : 0, duck: passed ? arc : 0, alpha, first };
}

/** How far row `i` has eased into its capsule. */
const morphK = (i: number, lt: number): number => swiftInOut(progress(MORPH + i * MORPH_EACH, MORPH + i * MORPH_EACH + MORPH_DUR, lt));

/**
 * The camera's lean into each finished chart's hold: 0 until the hold
 * starts, easing in toward HOLD_ZOOM over it, then back to exactly 0 over
 * its last beat, by the reset or the exit wipe.
 */
function holdZoom(m: ChartModel, lt: number): number {
  for (const [i, r] of m.races.entries()) {
    const from = r.last + HOLD_AFTER;
    const to = m.races[i + 1] ? RESET[0] : EXIT_WIPE[0];
    if (lt <= from || lt >= to) continue;
    const back = to - HOLD_BACK;
    if (back <= from) continue;
    return HOLD_ZOOM * (lt < back ? inOutSine(progress(from, back, lt)) : 1 - inOutSine(progress(back, to, lt)));
  }
  return 0;
}

// Whip in: the chart's layers trail one another in from the right on the
// whip's bar line, the details first, then each row's labels, then the
// tracks, each overshooting a little and settling by b1.

const WHIP_D = 1800;
function whipTrack(delay: number, over: number): (lt: number) => number {
  const travel = 0.28;
  return keys([
    [delay, WHIP_D],
    [delay + travel, -over, outQuart],
    [delay + travel + 0.12, 0, inOutSine],
  ]);
}
const L_DETAIL = whipTrack(0, 10);
const L_LABELS = [1, 2, 3, 4].map((k) => whipTrack(k * SHUTTER, 14 + 3 * k));
const L_TRACKS = [3, 4, 5, 6].map((k) => whipTrack(k * SHUTTER, 20 + 3 * k));
const layerAt = (list: readonly ((lt: number) => number)[], i: number) => list[Math.min(i, list.length - 1)];

/**
 * Draw `draw` at its layer's offset, smeared along the whip while it is
 * fast: a crisp copy at the head and a continuous blur behind it, faint
 * copies at most SMEAR_STEP px apart whose alphas taper to nothing and sum
 * to SMEAR_INK, so a solid shape trails a soft streak and a glyph a haze.
 */
const SMEAR_STEP = 4;
const SMEAR_MAX = 90;
const SMEAR_INK = 0.9;
function inLayer(ctx: CanvasRenderingContext2D, x: (lt: number) => number, lt: number, draw: () => void): void {
  const off = x(lt);
  if (off >= WHIP_D) return;
  const v = off - x(lt - SHUTTER);
  ctx.save();
  if (Math.abs(v) >= 20) {
    const trail = Math.max(-360, v);
    const n = Math.min(SMEAR_MAX, Math.ceil(Math.abs(trail) / SMEAR_STEP));
    // Tapering linearly from the head: sum over i of (1 - i / (n + 1)) is n / 2.
    const a0 = (2 * SMEAR_INK) / n;
    for (let i = n; i >= 1; i--) {
      ctx.save();
      ctx.translate(off - (trail * i) / n, 0);
      ctx.globalAlpha *= a0 * (1 - i / (n + 1));
      draw();
      ctx.restore();
    }
  }
  ctx.translate(off, 0);
  draw();
  ctx.restore();
}

// Drawing helpers.

/** A wipe's soft edge, px: what it wipes fades from nothing at its edge to full this far behind it. */
const FEATHER = 120;

/**
 * Where a left-to-right wipe over [a, b] has reached: nothing left of it
 * shows. It sets out a feather left of `from`, so the first frame of a wipe
 * dims nothing right of `from`.
 */
function wipeEdge(lt: number, a: number, z: number, from = 100, to = 1840): number {
  const p = progress(a, z, lt);
  return p <= 0 ? -1e5 : lerp(from - FEATHER, to, inOutSine(p));
}

// Wiped layers are drawn on a scratch canvas the size of the frame's, one
// per nesting level, cleared before each use, so no frame sees another's.
const scratch: HTMLCanvasElement[] = [];
let depth = 0;

/**
 * Draw `draw` wiped away left of `edge`, with a soft edge: into a scratch
 * layer, masked by a gradient from transparent at the edge to opaque
 * FEATHER px right of it, then laid over the frame. With no wipe under
 * way (edge at -1e5) it draws straight onto the frame.
 */
function wiped(ctx: CanvasRenderingContext2D, edge: number, draw: (c: CanvasRenderingContext2D) => void): void {
  if (edge <= -1e4) {
    draw(ctx);
    return;
  }
  const { width, height } = ctx.canvas;
  const layer = (scratch[depth] ??= makeCanvas(width, height));
  if (layer.width !== width || layer.height !== height) {
    layer.width = width;
    layer.height = height;
  }
  const c = layer.getContext("2d")!;
  c.save();
  c.setTransform(1, 0, 0, 1, 0, 0);
  c.clearRect(0, 0, width, height);
  c.setTransform(ctx.getTransform());
  depth++;
  try {
    draw(c);
  } finally {
    depth--;
  }
  c.globalCompositeOperation = "destination-in";
  const g = c.createLinearGradient(edge, 0, edge + FEATHER, 0);
  g.addColorStop(0, "rgba(0, 0, 0, 0)");
  g.addColorStop(1, "rgba(0, 0, 0, 1)");
  c.fillStyle = g;
  c.fillRect(-1e4, -1e4, 2e4 + 1e5, 2e4 + 1e5);
  c.restore();
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.drawImage(layer, 0, 0);
  ctx.restore();
}

/** A flash that is full on the first frame at or after `at` and then decays. */
const flash = (lt: number, at: number, decay: number): number => (lt < at ? 0 : Math.exp(-Math.max(0, lt - at - FRAME) / decay));

/** The font size that fits `text` in `width`, from `max` down to `floor`. */
function fitSize(ctx: CanvasRenderingContext2D, text: string, weight: number, max: number, floor: number, width: number, family?: string): number {
  const w = layout(ctx, text, font(max, weight, family)).width;
  return w <= width ? max : Math.max(floor, Math.floor((max * width) / w));
}

/** A softly lit rectangle's glow, sized in logical px at any output scale. */
function withGlow(ctx: CanvasRenderingContext2D, color: string, blur: number, draw: () => void): void {
  const m = ctx.getTransform();
  ctx.save();
  ctx.shadowColor = color;
  ctx.shadowBlur = blur * Math.hypot(m.a, m.b);
  draw();
  ctx.restore();
}

interface BarStyle {
  hk: boolean;
  fill: string;
  alpha: number;
}

const barStyle = (key: string): BarStyle =>
  key === "hk" ? { hk: true, fill: PALETTE.logo, alpha: 1 } : { hk: false, fill: PALETTE.text3, alpha: 0.35 };

/**
 * A bar from the axis to `x1` on row `cy`, `h` high with corner radius `r`:
 * hk's in logo cyan with its glow, the others text3 at 0.35 on an opaque
 * base, so they read the same over a track as on the bare stage.
 */
function drawBar(ctx: CanvasRenderingContext2D, key: string, x1: number, cy: number, h: number, r: number, glowK: number, alpha = 1): void {
  const w = x1 - X0;
  if (w <= 0.25 || alpha <= 0) return;
  const s = barStyle(key);
  ctx.save();
  ctx.globalAlpha *= alpha;
  const path = () => roundedRect(ctx, X0, cy - h / 2, w, h, r);
  if (s.hk) {
    const paint = () => {
      path();
      ctx.fillStyle = s.fill;
      ctx.fill();
    };
    if (glowK > 0.01) withGlow(ctx, rgba(PALETTE.logo, 0.55 * glowK), 30, paint);
    else paint();
  } else {
    path();
    ctx.fillStyle = PALETTE.bg;
    ctx.fill();
    ctx.fillStyle = rgba(s.fill, s.alpha);
    ctx.fill();
  }
  ctx.restore();
}

/** Light at a running bar's front: a hot edge and light pooled behind it, inside the bar. */
function tipLight(ctx: CanvasRenderingContext2D, x: number, cy: number, h: number, a: number, color: string): void {
  if (a <= 0.01 || x - X0 < 2) return;
  ctx.save();
  roundedRect(ctx, X0, cy - h / 2, x - X0, h, BAR_R);
  ctx.clip();
  ctx.globalCompositeOperation = "lighter";
  const g = ctx.createLinearGradient(x - 170, 0, x, 0);
  g.addColorStop(0, rgba(color, 0));
  g.addColorStop(0.7, rgba(color, 0.14 * a));
  g.addColorStop(1, rgba(color, 0.42 * a));
  ctx.fillStyle = g;
  ctx.fillRect(x - 170, cy - h / 2, 170, h);
  ctx.fillStyle = rgba(PALETTE.paper, 0.8 * a);
  ctx.fillRect(x - 3.5, cy - h / 2, 3.5, h);
  ctx.restore();
  glow(ctx, x - 4, cy, 46, color, 0.22 * a);
}

/** The flare where a bar stops: a hot edge a little taller than the bar (`h` high) and a bloom, on envelope `k`. */
function stopFlare(ctx: CanvasRenderingContext2D, x: number, cy: number, h: number, k: number, color: string): void {
  if (k <= 0.01) return;
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  glow(ctx, x - 4, cy, 74, color, 0.5 * k);
  ctx.fillStyle = rgba(color, 0.9 * k);
  const hh = (h / 2) * (1.12 + 0.2 * (1 - k));
  ctx.fillRect(x - 1.5, cy - hh, 3, hh * 2);
  ctx.restore();
}

/** A light band crossing a bar `h` high left to right at `u` 0..1, laid over it (never added, so no channel clips). */
function glint(ctx: CanvasRenderingContext2D, x1: number, cy: number, h: number, u: number, strength: number): void {
  if (u <= 0 || u >= 1 || x1 - X0 < 4) return;
  const cx = lerp(X0 - 80, x1 + 120, inOutSine(u));
  ctx.save();
  roundedRect(ctx, X0, cy - h / 2, x1 - X0, h, BAR_R);
  ctx.clip();
  ctx.transform(1, 0, -0.45, 1, 0.45 * cy, 0);
  const g = ctx.createLinearGradient(cx - 90, 0, cx + 90, 0);
  g.addColorStop(0, rgba(PALETTE.paper, 0));
  g.addColorStop(0.5, rgba(PALETTE.paper, strength));
  g.addColorStop(1, rgba(PALETTE.paper, 0));
  ctx.fillStyle = g;
  ctx.fillRect(cx - 90, cy - h / 2, 180, h);
  ctx.restore();
}

/** The median label's font at `size` px: hk's bold. */
const medianFont = (e: Entry, size: number): string => font(size, e.key === "hk" ? 700 : 400, MONO);

/** The median label's box at `size` px: where it sits for a bar ending at `x1`, and whether it had to go inside. */
function medianBox(ctx: CanvasRenderingContext2D, e: Entry, x1: number, size: number): { x: number; w: number; inside: boolean } {
  const w = layout(ctx, e.shown, medianFont(e, size)).width;
  const out = x1 + MEDIAN_GAP;
  return out + w > MEDIAN_LIMIT ? { x: x1 - MEDIAN_GAP - w, w, inside: true } : { x: out, w, inside: false };
}

// The chart's parts.

function drawTracks(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const n = raceAt(m, lt).order.length;
  const sweep = m.races.length > 1 ? progress(RESET[1] - b(0.1), m.races[1].start, lt) : 0;
  const h = m.geom.barH;
  for (let i = 0; i < Math.max(n, m.races[0].order.length); i++) {
    const cy = slotY(i, n);
    inLayer(ctx, layerAt(L_TRACKS, i), lt, () => {
      ctx.save();
      roundedRect(ctx, X0, cy - h / 2, TRACK_W, h, BAR_R);
      ctx.fillStyle = rgba(PALETTE.elevated, 0.62);
      ctx.fill();
      // The second race's scale: a light runs down the tracks as it arrives.
      if (sweep > 0 && sweep < 1) {
        ctx.clip();
        const cx = lerp(X0 - 200, X0 + TRACK_W + 200, inOutSine(clamp(sweep * 1.15 - i * 0.05)));
        const g = ctx.createLinearGradient(cx - 160, 0, cx + 160, 0);
        g.addColorStop(0, rgba(PALETTE.cyan, 0));
        g.addColorStop(0.5, rgba(PALETTE.cyan, 0.12));
        g.addColorStop(1, rgba(PALETTE.cyan, 0));
        ctx.fillStyle = g;
        ctx.fillRect(cx - 160, cy - h / 2, 320, h);
      }
      ctx.restore();
    });
  }
}

/** The start: a gate of cyan light drops down the axis on the race's first beat, and each stub flares as it passes. */
function drawStartFlash(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  for (const r of m.races) {
    const u = progress(r.start, r.start + 0.24, lt);
    if (u <= 0 || u >= 1) continue;
    const n = r.order.length;
    const top = slotY(0, n) - m.geom.barH / 2 - 10;
    const bot = slotY(n - 1, n) + m.geom.barH / 2 + 10;
    const drop = swiftOut(clamp(u / 0.35));
    const head = lerp(top, bot, drop);
    const a = (1 - u) ** 1.5;
    ctx.save();
    ctx.globalCompositeOperation = "lighter";
    const g = ctx.createLinearGradient(0, top, 0, head);
    g.addColorStop(0, rgba(PALETTE.cyan, 0.25 * a));
    g.addColorStop(1, rgba(PALETTE.cyanBright, 0.9 * a));
    ctx.fillStyle = g;
    ctx.fillRect(X0 - 2, top, 4, Math.max(0, head - top));
    glow(ctx, X0, head, 56, PALETTE.cyan, 0.6 * a);
    for (let i = 0; i < n; i++) {
      const cy = slotY(i, n);
      const hit = progress(top, bot, cy);
      const k = drop >= hit ? (1 - u) ** 2 : 0;
      glow(ctx, X0 + 8, cy, 64, PALETTE.cyan, 0.4 * k);
    }
    ctx.restore();
  }
}

interface BarNow {
  key: string;
  cy: number;
  x1: number;
  alpha: number;
  entry: Entry | undefined;
  race: RaceView;
}

/** A waiting stub's width: crouching into the start, then left behind by the bar. */
function nubAt(m: ChartModel, lt: number): number {
  const next = m.races.find((r) => lt < r.start + 0.1);
  if (!next) return NUB;
  return NUB * (1 - 0.4 * smoothstep(next.start - CROUCH, next.start, lt));
}

/** Every subject's bar as it stands at `lt`, before the exit. */
function barsAt(m: ChartModel, lt: number): BarNow[] {
  const race = raceAt(m, lt);
  const nub = nubAt(m, lt);
  return m.keys.map((key) => {
    const pos = rowPos(m, key, lt);
    const len = barLen(m, key, lt);
    return { key, cy: pos.y, x1: X0 + Math.max(len, nub), alpha: pos.alpha, entry: race.entries[key], race };
  });
}

function drawBars(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const last = lastRace(m);
  const fade = 1 - progress(MORPH, REST, lt);
  for (const bar of barsAt(m, lt)) {
    const { key, entry, race } = bar;
    const s = barStyle(key);
    // Rides its track's whip layer while the chart arrives.
    const layer = layerAt(L_TRACKS, rowPos(m, key, lt).first);
    let x1 = bar.x1;
    let r = BAR_R;
    let alpha = bar.alpha;
    let cy = bar.cy;
    let h0 = m.geom.barH;
    // The exit: into its capsule, or away if the capsules have no row for it.
    // With more rows than capsules, the ones that stay also close onto the capsules' rows.
    const row = last.entries[key]?.row ?? -1;
    if (lt >= MORPH) {
      if (row >= 0 && row < CAPSULES.rows.length) {
        const k = morphK(row, lt);
        x1 = lerp(x1, CAPSULES.x1, k);
        r = lerp(BAR_R, CAP_R, k);
        cy = lerp(cy, CAPSULES.rows[row], k);
        h0 = lerp(h0, CAPSULES.h, k);
      } else {
        alpha *= fade;
      }
    }
    // A small swell as it stops, the bar's only secondary motion: its length never leaves the median.
    const stop = entry && lt >= race.start ? entry.stop : null;
    const swell = stop === null ? 0 : Math.sin(Math.PI * progress(stop, stop + 0.18, lt)) * (1 - progress(stop, stop + 0.18, lt));
    const h = h0 * (1 + 0.16 * swell);
    const hold = hkGlow(m, lt);
    inLayer(ctx, layer, lt, () => {
      drawBar(ctx, key, x1, cy, h, r, s.hk ? hold * fade : 0, alpha);
      if (!entry || lt < race.start || lt >= MORPH) return;
      const running = lt < entry.stop;
      if (running) tipLight(ctx, x1, cy, h, progress(race.start, race.start + 0.04, lt) * alpha, s.hk ? PALETTE.glint : PALETTE.text1);
      stopFlare(ctx, x1, cy, h0, flash(lt, entry.stop, 0.07) * alpha, s.hk ? PALETTE.glint : PALETTE.text1);
    });
  }
}

/**
 * hk's glow: steady while it waits, brightening as it runs, flaring as it
 * stops, then breathing slowly while the chart holds; back to steady over
 * the reset, so it never jumps.
 */
function hkGlow(m: ChartModel, lt: number): number {
  let level = 0.8;
  m.races.forEach((r, i) => {
    const e = r.entries.hk;
    if (!e || lt < r.start) return;
    const since = lt - e.stop;
    const breathe = since > 0 ? 0.12 * Math.sin((Math.PI * since) / b(2)) ** 2 : 0;
    level = 0.8 + 0.2 * progress(r.start, e.stop, lt) - breathe + 0.35 * flash(lt, e.stop, 0.12);
    if (m.races[i + 1] && lt >= RESET[0]) level = lerp(level, 0.8, smoothstep(RESET[0], RESET[1], lt));
  });
  return level;
}

/**
 * The glints: across hk's bar after each bracket; with one race, a softer
 * one across it midway through the long hold, and one down every bar as
 * the hold's caption lands.
 */
function drawGlints(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  if (lt >= EXIT_WIPE[0]) return;
  const bars = barsAt(m, lt);
  const h = m.geom.barH;
  const hk = bars.find((x) => x.key === "hk");
  for (const r of m.races) {
    const at = r.last + GLINT_AFTER;
    if (hk && raceAt(m, lt) === r) glint(ctx, hk.x1, hk.cy, h, progress(at, at + GLINT_LEN, lt), 0.34);
  }
  if (m.races.length === 1) {
    if (hk) glint(ctx, hk.x1, hk.cy, h, progress(HOLD_GLINT, HOLD_GLINT + HOLD_GLINT_LEN, lt), 0.24);
    for (const [i, bar] of bars.entries()) {
      const at = ROLL_CALL.from + i * ROLL_CALL.each;
      glint(ctx, bar.x1, bar.cy, h, progress(at, at + ROLL_CALL.len, lt), bar.key === "hk" ? 0.34 : 0.16);
    }
  }
}

/** Whiskers, medians and the bracket: they come with each stop, and the first race's go with the reset's wipe. */
function drawFigures(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const race = raceAt(m, lt);
  if (lt < race.start) return;
  const reset = race === m.races[0] && m.races.length > 1 ? wipeEdge(lt, RESET_WIPE[0], RESET_WIPE[1], X0 - 20) : -1e5;
  wiped(ctx, reset, (c) => figures(c, m, race, lt));
}

function figures(ctx: CanvasRenderingContext2D, m: ChartModel, race: RaceView, lt: number): void {
  const g = m.geom;
  // Figures stay where their bars stopped while the bars reel back under the wipe.
  const bars = barsAt(m, lt).map((x) => (x.entry && lt >= x.entry.stop ? { ...x, x1: X0 + x.entry.len } : x));
  for (const bar of bars) {
    const e = bar.entry;
    if (!e || lt < e.stop) continue;
    const x1 = bar.x1;
    if (g.detail) whisker(ctx, e, x1, bar.cy + WHISKER_DY, bar.alpha, lt);

    // The median pops as its bar stops.
    const box = medianBox(ctx, e, x1, g.median);
    const s = popIn(lt, e.stop, 4.8, 0.52);
    const a = clamp((lt - e.stop) / 0.04) * bar.alpha;
    if (a > 0 && s > 0.01) {
      const hk = e.key === "hk";
      const fill = box.inside ? (hk ? PALETTE.night : PALETTE.text1) : hk ? PALETTE.text1 : PALETTE.text2;
      ctx.save();
      ctx.globalAlpha *= a;
      const ax = box.inside ? box.x + box.w : box.x;
      ctx.translate(ax, bar.cy);
      ctx.scale(s, s);
      drawText(ctx, e.shown, box.inside ? -box.w : 0, 0.35 * g.median, { font: medianFont(e, g.median), fill: mix(fill, PALETTE.paper, 0.7 * flash(lt, e.stop, 0.08)) });
      ctx.restore();
    }
  }
  drawBracket(ctx, race, bars, lt, g);
}

/**
 * The min–max whisker at `y`, drawing out of the bar's end (`x1`) to its
 * ends after the stop. A spread narrower than a tick is one short
 * round-capped dash, at least 6 px, centred on it, so it never reads as a
 * speck or a letter.
 */
function whisker(ctx: CanvasRenderingContext2D, e: Entry, x1: number, y: number, alpha: number, lt: number): void {
  const k = swiftOut(progress(e.stop + 0.02, e.stop + 0.26, lt));
  const tight = e.hi - e.lo < TICK;
  let lo0 = X0 + e.lo;
  let hi0 = X0 + e.hi;
  if (tight) {
    const mid = (lo0 + hi0) / 2;
    const half = Math.max(3, (hi0 - lo0) / 2);
    lo0 = mid - half;
    hi0 = mid + half;
  }
  const lo = lerp(x1, lo0, k);
  const hi = lerp(x1, hi0, k);
  ctx.save();
  ctx.globalAlpha *= alpha;
  ctx.strokeStyle = rgba(PALETTE.text1, 0.45);
  ctx.lineWidth = 3;
  ctx.lineCap = tight ? "round" : "butt";
  ctx.beginPath();
  ctx.moveTo(lo, y);
  ctx.lineTo(hi, y);
  // A narrowish spread gets shorter ticks.
  const tk = tight ? 0 : (Math.min(TICK, Math.max(8, 0.7 * (e.hi - e.lo))) / 2) * swiftOut(progress(0.7, 1, k));
  if (tk > 0.2) {
    ctx.moveTo(lo, y - tk);
    ctx.lineTo(lo, y + tk);
    ctx.moveTo(hi, y - tk);
    ctx.lineTo(hi, y + tk);
  }
  ctx.stroke();
  ctx.restore();
}

/** The bracket from hk's bar end to the fastest rival's, 40 px above hk's row, with a guide down to the rival's bar. */
function drawBracket(ctx: CanvasRenderingContext2D, race: RaceView, bars: readonly BarNow[], lt: number, g: RowGeom): void {
  const at = race.last;
  if (lt < at) return;
  const hk = bars.find((x) => x.key === "hk");
  const rival = bars.find((x) => x.key === race.rival);
  if (!hk || !rival || !hk.entry || !rival.entry) return;
  const y = hk.cy - BRACKET_DY;
  const xa = hk.x1;
  const xb = rival.x1;
  const tick = swiftOut(progress(at, at + b(0.1), lt));
  const span = swiftInOut(progress(at + b(0.03), at + b(0.2), lt));
  // The far tick grows as the near one did, as the span arrives.
  const tickB = swiftOut(progress(at + b(0.18), at + b(0.28), lt));
  const guide = swiftOut(progress(at + b(0.1), at + b(0.25), lt));
  ctx.save();
  ctx.strokeStyle = PALETTE.cyan;
  ctx.lineCap = "round";
  ctx.lineWidth = 3;
  ctx.beginPath();
  const th = (TICK / 2) * tick;
  ctx.moveTo(xa, y - th);
  ctx.lineTo(xa, y + th);
  if (span > 0) {
    ctx.moveTo(xa, y);
    ctx.lineTo(lerp(xa, xb, span), y);
  }
  if (tickB > 0) {
    const tb = (TICK / 2) * tickB;
    ctx.moveTo(xb, y - tb);
    ctx.lineTo(xb, y + tb);
  }
  ctx.stroke();
  // The guide drops to the rival's bar, broken around hk's median label.
  if (guide > 0) {
    const foot = rival.cy - g.barH / 2 - 8;
    const bottom = lerp(y + TICK / 2, foot, guide);
    const box = medianBox(ctx, hk.entry, xa, g.median);
    const em = g.median / MEDIAN_SIZE;
    const gap = xb > box.x - 10 && xb < box.x + box.w + 10 ? [hk.cy - 26 * em, hk.cy + 24 * em] : null;
    ctx.globalAlpha *= 0.55;
    ctx.lineWidth = 2;
    ctx.setLineDash([6, 7]);
    ctx.beginPath();
    const top = y + TICK / 2 + 4;
    if (gap && bottom > gap[0]) {
      ctx.moveTo(xb, top);
      ctx.lineTo(xb, gap[0]);
      if (bottom > gap[1]) {
        ctx.moveTo(xb, gap[1]);
        ctx.lineTo(xb, bottom);
      }
    } else if (bottom > top) {
      ctx.moveTo(xb, top);
      ctx.lineTo(xb, bottom);
    }
    ctx.stroke();
    ctx.setLineDash([]);
  }
  ctx.restore();
  // A spark rides the span's head as it draws, fading over its last tenth.
  if (span > 0 && span < 1) {
    const k: Curve = { a: { x: xa, y }, c: { x: (xa + xb) / 2, y }, b: { x: xb, y } };
    drawSpark(ctx, k, span, { color: PALETTE.cyanBright, size: 5, trail: 0.3, alpha: 1 - progress(0.9, 1, span) });
  }
}

/**
 * A halo of the stage's colour around `text`, placed as drawText places it
 * left-aligned at (x, y), at strength `k`: letters drawn over it read in
 * front of whatever it covers.
 */
function haloText(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, f: string, k: number): void {
  const t = ctx.getTransform();
  ctx.save();
  ctx.font = f;
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
  ctx.lineJoin = "round";
  ctx.lineWidth = 14;
  ctx.strokeStyle = rgba(PALETTE.bg, k);
  ctx.shadowColor = rgba(PALETTE.bg, k);
  ctx.shadowBlur = 16 * Math.hypot(t.a, t.b);
  ctx.strokeText(text, x, y);
  ctx.restore();
}

/** Names and modes, each row on its label layer, kicking as its bar stops. */
function drawLabels(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const race = raceAt(m, lt);
  const [A, B] = m.races;
  const g = m.geom;
  const nameFont = font(g.name, 600);
  // A row that jumps draws last, in front of the rows it passes.
  const rows = m.keys.map((key) => ({ key, pos: rowPos(m, key, lt) })).sort((p, q) => p.pos.jump - q.pos.jump);
  for (const { key, pos } of rows) {
    if (pos.alpha <= 0) continue;
    const e = race.entries[key] ?? A.entries[key] ?? B?.entries[key];
    if (!e) continue;
    const hk = key === "hk";
    const kick = lt >= race.start && race.entries[key] ? pulse(lt, race.entries[key].stop, 0.012, 0.1) : 0;
    const jump = pos.jump;
    // Its halo comes up fast, so it is whole before it reaches the next row.
    const halo = clamp(3 * jump);
    inLayer(ctx, layerAt(L_LABELS, pos.first), lt, () => {
      ctx.save();
      // A tool climbing two places jumps them: out to the right and up, a little larger,
      // in front, while the rows it overtakes duck back and dim under it.
      ctx.globalAlpha *= pos.alpha * (1 - 0.75 * pos.duck);
      ctx.translate(NAME_X + 90 * jump - 18 * pos.duck, pos.y);
      const s = 1 + 0.05 * kick + 0.08 * jump - 0.04 * pos.duck;
      ctx.scale(s, s);
      const base = hk ? PALETTE.logo : PALETTE.text1;
      if (halo > 0.01) haloText(ctx, e.label, 0, 2, nameFont, halo);
      drawText(ctx, e.label, 0, 2, { font: nameFont, fill: kick > 0.02 ? mix(base, PALETTE.paper, 0.65 * kick) : base });
      // The mode: swapped with a card flip where the second race's differs.
      const ma = A.entries[key]?.mode ?? e.mode;
      const mb = B?.entries[key]?.mode ?? ma;
      const flip = ma === mb ? 1 : progress(REORDER[0], REORDER[0] + b(0.25), lt);
      const text = flip < 0.5 ? ma : mb;
      const sy = ma === mb ? 1 : Math.abs(Math.cos(Math.PI * flip));
      if (g.detail && sy > 0.02) {
        const size = fitSize(ctx, text, 500, MODE_SIZE, MODE_FLOOR, MODE_MAX_W);
        ctx.save();
        ctx.translate(0, MODE_DY - size * 0.35);
        ctx.scale(1, sy);
        // Edge-on, the card's halo would only cut a dark band through the row behind.
        if (halo * sy > 0.01) haloText(ctx, text, 0, size * 0.35, font(size, 500), halo * sy);
        drawText(ctx, text, 0, size * 0.35, { font: font(size, 500), fill: PALETTE.text3 });
        ctx.restore();
      }
      ctx.restore();
    });
  }
}

/** A detail line rising into place over [at, at + 0.25 s] (the reel's word motion, as one line). */
function riseLine(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, lt: number, at: number, align: "left" | "right", size: number): void {
  const p = progress(at, at + 0.25, lt);
  if (p <= 0) return;
  ctx.save();
  ctx.globalAlpha *= clamp(p / 0.6);
  drawText(ctx, text, x, y + 12 * (1 - swiftOut(p)), { font: font(size, 500), fill: PALETTE.text3, align });
  ctx.restore();
}

function drawDetails(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const [A, B] = m.races;
  inLayer(ctx, L_DETAIL, lt, () => {
    drawText(ctx, m.workload, WORKLOAD_AT.x, WORKLOAD_AT.y, { font: DETAIL_FONT, fill: PALETTE.text3 });
    const size = (s: string) => fitSize(ctx, s, 500, 40, 30, DETAIL_MAX_W);
    const summaryA = (c: CanvasRenderingContext2D) => drawText(c, A.race.summary, SUMMARY_AT.x, SUMMARY_AT.y, { font: font(size(A.race.summary), 500), fill: PALETTE.text3 });
    if (!B || lt < RESET_WIPE[0]) {
      summaryA(ctx);
      return;
    }
    // The summary swaps: the first wipes away, the second rises.
    if (lt < RESET_WIPE[1]) wiped(ctx, wipeEdge(lt, RESET_WIPE[0], RESET_WIPE[1], SUMMARY_AT.x - 10, 1780), summaryA);
    riseLine(ctx, B.race.summary, SUMMARY_AT.x, SUMMARY_AT.y, lt, RESET_WIPE[1] - b(0.05), "left", size(B.race.summary));
  });
}

/** The bottom detail, rising on FOOT_IN (FOOT_IN_ONE with one race), fitted between the axis and the margin. */
function drawFoot(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const at = m.races.length > 1 ? FOOT_IN : FOOT_IN_ONE;
  if (lt < at) return;
  riseLine(ctx, FOOT, FOOT_AT.x, FOOT_AT.y, lt, at, "right", fitSize(ctx, FOOT, 500, 40, 30, FOOT_MAX_W));
}

/**
 * The chart at `lt` (after the whip's bar line and before REST): tracks,
 * bars, figures, labels and details, back to front. The bottom detail is
 * drawn outside the camera's lean, so it always ends on the margin.
 */
export function drawChart(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  ctx.save();
  // While a finished chart holds, the camera leans in.
  const z = 1 + holdZoom(m, lt);
  if (z !== 1) {
    ctx.translate(HOLD_ANCHOR.x, HOLD_ANCHOR.y);
    ctx.scale(z, z);
    ctx.translate(-HOLD_ANCHOR.x, -HOLD_ANCHOR.y);
  }
  // The tracks, figures, labels and details go with the exit wipe; the bars stay for the capsules.
  const exit = wipeEdge(lt, EXIT_WIPE[0], EXIT_WIPE[1]);
  wiped(ctx, exit, (c) => drawTracks(c, m, lt));
  drawStartFlash(ctx, m, lt);
  drawBars(ctx, m, lt);
  drawGlints(ctx, m, lt);
  wiped(ctx, exit, (c) => {
    drawFigures(c, m, lt);
    drawLabels(c, m, lt);
    drawDetails(c, m, lt);
  });
  // Capsules the chart has no bar for grow in from the axis.
  for (let i = lastRace(m).order.length; i < CAPSULES.rows.length; i++) growCapsule(ctx, i, lt);
  ctx.restore();
  wiped(ctx, exit, (c) => drawFoot(c, m, lt));
}

/** Capsules grow in over this long each, a 64th apart, the last home on REST. */
export const GROW_DUR = b(0.5);
export const GROW_EACH = b(1 / 16);
export const GROW_AT = REST - GROW_DUR - (CAPSULES.rows.length - 1) * GROW_EACH;

/**
 * Capsule `i` of race|morph growing out of the axis, from nothing to its
 * full length with a slight overshoot, landing exactly on it `dur` after
 * `at` (by default, a 64th apart, the last by REST).
 */
export function growCapsule(ctx: CanvasRenderingContext2D, i: number, lt: number, at = GROW_AT + i * GROW_EACH, dur = GROW_DUR): void {
  const k = land(lt, at, dur, 0.06);
  if (k <= 0) return;
  const w = (CAPSULES.x1 - CAPSULES.x0) * k;
  const cy = CAPSULES.rows[i];
  ctx.save();
  ctx.globalAlpha *= CAPSULES.alpha[i] * clamp(k * 4);
  roundedRect(ctx, CAPSULES.x0, cy - CAP_R, w, CAPSULES.h, CAP_R);
  ctx.fillStyle = CAPSULES.color[i];
  ctx.fill();
  ctx.restore();
}
