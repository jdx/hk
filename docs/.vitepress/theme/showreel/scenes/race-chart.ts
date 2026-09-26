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
// summary swap. The labels, figures and details wipe on b14.5, and the bars
// ease into race|morph's four capsules by b15.75.
//
// Everything here is a pure function of the section's local time and the
// facts; the model is cached by the facts object alone.

import { BEAT, PALETTE } from "../bible";
import { mix, rgba } from "../color";
import type { Race, ReelFacts } from "../facts";
import { glow, roundedRect } from "../fx";
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
const NAME_FONT = font(56, 600);
/** Modes shrink to fit left of the axis, from 40 px to a 30 px floor; never abbreviated. */
const MODE_SIZE = 40;
const MODE_FLOOR = 30;
const MODE_MAX_W = X0 - NAME_X - 28;
const DETAIL_FONT = font(40, 500);
const DETAIL_MAX_W = 1600;
const MEDIAN_SIZE = 40;
/** The median label's gap from its bar's end, and the x it must not pass. */
const MEDIAN_GAP = 16;
const MEDIAN_LIMIT = 1740;
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
const FOOT_AT = { x: 1760, y: 724 } as const;
/** The bottom detail: the page has every scenario, the commit one included. */
export const FOOT = "All tools and scenarios, including commits: hk.jdx.dev/benchmarks";

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
const CROUCH = b(0.25);
/** The bottom detail rises. */
export const FOOT_IN = b(12);
/** Labels, figures, whiskers, bracket, details and tracks wipe left to right. */
export const EXIT_WIPE = [b(14.5), b(15)] as const;
/** The bars ease into the capsules, a 64th apart, all home by b15.75. */
export const MORPH = b(15);
export const MORPH_EACH = b(1 / 16);
export const MORPH_DUR = b(0.5);
/** From here the frame is race|morph. */
export const REST = b(15.75);

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
  const m = races.length ? { workload: `${f.workload.files} files · ${f.workload.fixers} fixers · ${f.workload.cpus} CPUs`, races, keys } : null;
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
const slotY = (i: number, n: number): number => {
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

/** Draw `draw` at its layer's offset, smeared along the whip while it is fast. */
function inLayer(ctx: CanvasRenderingContext2D, x: (lt: number) => number, lt: number, draw: () => void): void {
  const off = x(lt);
  if (off >= WHIP_D) return;
  const v = off - x(lt - SHUTTER);
  ctx.save();
  if (Math.abs(v) < 20) {
    ctx.translate(off, 0);
    draw();
  } else {
    // Enough copies, close enough together, that the trail reads as a streak, not a stutter.
    const trail = Math.max(-360, v);
    const n = Math.min(18, Math.ceil(Math.abs(trail) / 12));
    for (let i = n - 1; i >= 0; i--) {
      ctx.save();
      ctx.translate(off - (trail * i) / n, 0);
      ctx.globalAlpha *= i === 0 ? 1 : 0.42 * (1 - i / n) ** 1.4;
      draw();
      ctx.restore();
    }
  }
  ctx.restore();
}

// Drawing helpers.

/** Clip away everything left of `edge` (a left-to-right wipe). */
function clipFrom(ctx: CanvasRenderingContext2D, edge: number): void {
  if (edge <= -1e4) return;
  ctx.beginPath();
  ctx.rect(edge, -1e4, 1e5, 2e4);
  ctx.clip();
}

/** Where a left-to-right wipe over [a, b] has reached: nothing left of it shows. */
function wipeEdge(lt: number, a: number, z: number, from = 100, to = 1840): number {
  const p = progress(a, z, lt);
  return p <= 0 ? -1e5 : lerp(from, to, inOutSine(p));
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

/** The flare where a bar stops: a hot edge a little taller than the bar and a bloom, on envelope `k`. */
function stopFlare(ctx: CanvasRenderingContext2D, x: number, cy: number, k: number, color: string): void {
  if (k <= 0.01) return;
  ctx.save();
  ctx.globalCompositeOperation = "lighter";
  glow(ctx, x - 4, cy, 74, color, 0.5 * k);
  ctx.fillStyle = rgba(color, 0.9 * k);
  const hh = (BAR_H / 2) * (1.12 + 0.2 * (1 - k));
  ctx.fillRect(x - 1.5, cy - hh, 3, hh * 2);
  ctx.restore();
}

/** A light band crossing a bar left to right at `u` 0..1, laid over it (never added, so no channel clips). */
function glint(ctx: CanvasRenderingContext2D, x1: number, cy: number, u: number, strength: number): void {
  if (u <= 0 || u >= 1 || x1 - X0 < 4) return;
  const cx = lerp(X0 - 80, x1 + 120, inOutSine(u));
  ctx.save();
  roundedRect(ctx, X0, cy - BAR_H / 2, x1 - X0, BAR_H, BAR_R);
  ctx.clip();
  ctx.transform(1, 0, -0.45, 1, 0.45 * cy, 0);
  const g = ctx.createLinearGradient(cx - 90, 0, cx + 90, 0);
  g.addColorStop(0, rgba(PALETTE.paper, 0));
  g.addColorStop(0.5, rgba(PALETTE.paper, strength));
  g.addColorStop(1, rgba(PALETTE.paper, 0));
  ctx.fillStyle = g;
  ctx.fillRect(cx - 90, cy - BAR_H / 2, 180, BAR_H);
  ctx.restore();
}

/** The median label's box: where it sits for a bar ending at `x1`, and whether it had to go inside. */
function medianBox(ctx: CanvasRenderingContext2D, e: Entry, x1: number): { x: number; w: number; inside: boolean } {
  const w = layout(ctx, e.shown, font(MEDIAN_SIZE, e.key === "hk" ? 700 : 400, MONO)).width;
  const out = x1 + MEDIAN_GAP;
  return out + w > MEDIAN_LIMIT ? { x: x1 - MEDIAN_GAP - w, w, inside: true } : { x: out, w, inside: false };
}

// The chart's parts.

function drawTracks(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const n = raceAt(m, lt).order.length;
  const sweep = m.races.length > 1 ? progress(RESET[1] - b(0.1), m.races[1].start, lt) : 0;
  for (let i = 0; i < Math.max(n, m.races[0].order.length); i++) {
    const cy = slotY(i, n);
    inLayer(ctx, layerAt(L_TRACKS, i), lt, () => {
      ctx.save();
      roundedRect(ctx, X0, cy - BAR_H / 2, SCALE, BAR_H, BAR_R);
      ctx.fillStyle = rgba(PALETTE.elevated, 0.62);
      ctx.fill();
      // The second race's scale: a light runs down the tracks as it arrives.
      if (sweep > 0 && sweep < 1) {
        ctx.clip();
        const cx = lerp(X0 - 200, X0 + SCALE + 200, inOutSine(clamp(sweep * 1.15 - i * 0.05)));
        const g = ctx.createLinearGradient(cx - 160, 0, cx + 160, 0);
        g.addColorStop(0, rgba(PALETTE.cyan, 0));
        g.addColorStop(0.5, rgba(PALETTE.cyan, 0.12));
        g.addColorStop(1, rgba(PALETTE.cyan, 0));
        ctx.fillStyle = g;
        ctx.fillRect(cx - 160, cy - BAR_H / 2, 320, BAR_H);
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
    const top = slotY(0, n) - BAR_H / 2 - 10;
    const bot = slotY(n - 1, n) + BAR_H / 2 + 10;
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
    // The exit: into its capsule, or away if the capsules have no row for it.
    const row = last.entries[key]?.row ?? -1;
    if (lt >= MORPH) {
      if (row >= 0 && row < CAPSULES.rows.length) {
        const k = morphK(row, lt);
        x1 = lerp(x1, CAPSULES.x1, k);
        r = lerp(BAR_R, CAP_R, k);
      } else {
        alpha *= fade;
      }
    }
    // A small swell as it stops, the bar's only secondary motion: its length never leaves the median.
    const stop = entry && lt >= race.start ? entry.stop : null;
    const swell = stop === null ? 0 : Math.sin(Math.PI * progress(stop, stop + 0.18, lt)) * (1 - progress(stop, stop + 0.18, lt));
    const h = BAR_H * (1 + 0.16 * swell);
    const hold = hkGlow(m, lt);
    inLayer(ctx, layer, lt, () => {
      drawBar(ctx, key, x1, bar.cy, h, r, s.hk ? hold * fade : 0, alpha);
      if (!entry || lt < race.start || lt >= MORPH) return;
      const running = lt < entry.stop;
      if (running) tipLight(ctx, x1, bar.cy, h, progress(race.start, race.start + 0.04, lt) * alpha, s.hk ? PALETTE.glint : PALETTE.text1);
      stopFlare(ctx, x1, bar.cy, flash(lt, entry.stop, 0.07) * alpha, s.hk ? PALETTE.glint : PALETTE.text1);
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

/** The glints: across hk's bar after each bracket, and down every bar as the one-claim hold's caption lands. */
function drawGlints(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  if (lt >= EXIT_WIPE[0]) return;
  const bars = barsAt(m, lt);
  for (const r of m.races) {
    const u = progress(r.last + b(0.25), r.last + b(0.25) + 0.36, lt);
    const hk = bars.find((x) => x.key === "hk");
    if (hk && raceAt(m, lt) === r) glint(ctx, hk.x1, hk.cy, u, 0.34);
  }
  if (m.races.length === 1) {
    for (const [i, bar] of bars.entries()) glint(ctx, bar.x1, bar.cy, progress(b(9.5) + i * b(0.125), b(9.5) + i * b(0.125) + 0.4, lt), bar.key === "hk" ? 0.34 : 0.16);
  }
}

/** Whiskers, medians and the bracket: they come with each stop and go with the wipes. */
function drawFigures(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const race = raceAt(m, lt);
  if (lt < race.start) return;
  ctx.save();
  if (race === m.races[0] && m.races.length > 1) clipFrom(ctx, wipeEdge(lt, RESET_WIPE[0], RESET_WIPE[1], X0 - 20));
  clipFrom(ctx, wipeEdge(lt, EXIT_WIPE[0], EXIT_WIPE[1]));
  // Figures stay where their bars stopped while the bars reel back under the wipe.
  const bars = barsAt(m, lt).map((x) => (x.entry && lt >= x.entry.stop ? { ...x, x1: X0 + x.entry.len } : x));
  for (const bar of bars) {
    const e = bar.entry;
    if (!e || lt < e.stop) continue;
    // The whisker draws out of the bar's end to its min and max.
    const k = swiftOut(progress(e.stop + 0.02, e.stop + 0.26, lt));
    const y = bar.cy + WHISKER_DY;
    const x1 = bar.x1;
    const lo = lerp(x1, X0 + e.lo, k);
    const hi = lerp(x1, X0 + e.hi, k);
    ctx.save();
    ctx.globalAlpha *= bar.alpha;
    ctx.strokeStyle = rgba(PALETTE.text1, 0.45);
    ctx.lineWidth = 3;
    ctx.lineCap = "butt";
    ctx.beginPath();
    ctx.moveTo(lo, y);
    ctx.lineTo(hi, y);
    // A tight spread gets shorter ticks, so a narrow whisker never reads as a letter.
    const tk = (Math.min(TICK, Math.max(8, 0.7 * (e.hi - e.lo))) / 2) * swiftOut(progress(0.7, 1, k));
    if (tk > 0.2) {
      ctx.moveTo(lo, y - tk);
      ctx.lineTo(lo, y + tk);
      ctx.moveTo(hi, y - tk);
      ctx.lineTo(hi, y + tk);
    }
    ctx.stroke();
    ctx.restore();

    // The median pops as its bar stops.
    const box = medianBox(ctx, e, x1);
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
      drawText(ctx, e.shown, box.inside ? -box.w : 0, 14, { font: font(MEDIAN_SIZE, hk ? 700 : 400, MONO), fill: mix(fill, PALETTE.paper, 0.7 * flash(lt, e.stop, 0.08)) });
      ctx.restore();
    }
  }
  drawBracket(ctx, race, bars, lt);
  ctx.restore();
}

/** The bracket from hk's bar end to the fastest rival's, 40 px above hk's row, with a guide down to the rival's bar. */
function drawBracket(ctx: CanvasRenderingContext2D, race: RaceView, bars: readonly BarNow[], lt: number): void {
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
  if (span >= 1) {
    ctx.moveTo(xb, y - TICK / 2);
    ctx.lineTo(xb, y + TICK / 2);
  }
  ctx.stroke();
  // The guide drops to the rival's bar, broken around hk's median label.
  if (guide > 0) {
    const foot = rival.cy - BAR_H / 2 - 8;
    const bottom = lerp(y + TICK / 2, foot, guide);
    const box = medianBox(ctx, hk.entry, xa);
    const gap = xb > box.x - 10 && xb < box.x + box.w + 10 ? [hk.cy - 26, hk.cy + 24] : null;
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
  // A spark rides the span's head as it draws.
  if (span > 0 && span < 1) {
    const k: Curve = { a: { x: xa, y }, c: { x: (xa + xb) / 2, y }, b: { x: xb, y } };
    drawSpark(ctx, k, span, { color: PALETTE.cyanBright, size: 5, trail: 0.3 });
  }
}

/** Names and modes, each row on its label layer, kicking as its bar stops. */
function drawLabels(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  const race = raceAt(m, lt);
  const [A, B] = m.races;
  ctx.save();
  clipFrom(ctx, wipeEdge(lt, EXIT_WIPE[0], EXIT_WIPE[1]));
  for (const key of m.keys) {
    const pos = rowPos(m, key, lt);
    if (pos.alpha <= 0) continue;
    const e = race.entries[key] ?? A.entries[key] ?? B?.entries[key];
    if (!e) continue;
    const hk = key === "hk";
    const kick = lt >= race.start && race.entries[key] ? pulse(lt, race.entries[key].stop, 0.012, 0.1) : 0;
    const jump = pos.jump;
    inLayer(ctx, layerAt(L_LABELS, pos.first), lt, () => {
      ctx.save();
      // A tool climbing two places jumps them: out to the right and up, a little larger,
      // while the rows it overtakes duck back and dim under it.
      ctx.globalAlpha *= pos.alpha * (1 - 0.5 * pos.duck);
      ctx.translate(NAME_X + 44 * jump - 10 * pos.duck, pos.y);
      const s = 1 + 0.05 * kick + 0.08 * jump - 0.04 * pos.duck;
      ctx.scale(s, s);
      const base = hk ? PALETTE.logo : PALETTE.text1;
      drawText(ctx, e.label, 0, 2, { font: NAME_FONT, fill: kick > 0.02 ? mix(base, PALETTE.paper, 0.65 * kick) : base });
      // The mode: swapped with a card flip where the second race's differs.
      const ma = A.entries[key]?.mode ?? e.mode;
      const mb = B?.entries[key]?.mode ?? ma;
      const flip = ma === mb ? 1 : progress(REORDER[0], REORDER[0] + b(0.25), lt);
      const text = flip < 0.5 ? ma : mb;
      const sy = ma === mb ? 1 : Math.abs(Math.cos(Math.PI * flip));
      if (sy > 0.02) {
        const size = fitSize(ctx, text, 500, MODE_SIZE, MODE_FLOOR, MODE_MAX_W);
        ctx.save();
        ctx.translate(0, 44 - size * 0.35);
        ctx.scale(1, sy);
        drawText(ctx, text, 0, size * 0.35, { font: font(size, 500), fill: PALETTE.text3 });
        ctx.restore();
      }
      ctx.restore();
    });
  }
  ctx.restore();
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
  ctx.save();
  clipFrom(ctx, wipeEdge(lt, EXIT_WIPE[0], EXIT_WIPE[1]));
  const [A, B] = m.races;
  inLayer(ctx, L_DETAIL, lt, () => {
    drawText(ctx, m.workload, WORKLOAD_AT.x, WORKLOAD_AT.y, { font: DETAIL_FONT, fill: PALETTE.text3 });
    const size = (s: string) => fitSize(ctx, s, 500, 40, 30, DETAIL_MAX_W);
    if (!B || lt < RESET_WIPE[0]) {
      drawText(ctx, A.race.summary, SUMMARY_AT.x, SUMMARY_AT.y, { font: font(size(A.race.summary), 500), fill: PALETTE.text3 });
      return;
    }
    // The summary swaps: the first wipes away, the second rises.
    ctx.save();
    clipFrom(ctx, wipeEdge(lt, RESET_WIPE[0], RESET_WIPE[1], SUMMARY_AT.x - 10, 1780));
    drawText(ctx, A.race.summary, SUMMARY_AT.x, SUMMARY_AT.y, { font: font(size(A.race.summary), 500), fill: PALETTE.text3 });
    ctx.restore();
    riseLine(ctx, B.race.summary, SUMMARY_AT.x, SUMMARY_AT.y, lt, RESET_WIPE[1] - b(0.05), "left", size(B.race.summary));
  });
  riseLine(ctx, FOOT, FOOT_AT.x, FOOT_AT.y, lt, FOOT_IN, "right", 40);
  ctx.restore();
}

/**
 * The chart at `lt` (after the whip's bar line and before REST): tracks,
 * bars, figures, labels and details, back to front.
 */
export function drawChart(ctx: CanvasRenderingContext2D, m: ChartModel, lt: number): void {
  ctx.save();
  // The tracks go with the exit wipe; the bars stay for the capsules.
  ctx.save();
  clipFrom(ctx, wipeEdge(lt, EXIT_WIPE[0], EXIT_WIPE[1]));
  drawTracks(ctx, m, lt);
  ctx.restore();
  drawStartFlash(ctx, m, lt);
  drawBars(ctx, m, lt);
  drawGlints(ctx, m, lt);
  drawFigures(ctx, m, lt);
  drawLabels(ctx, m, lt);
  drawDetails(ctx, m, lt);
  // Capsules the chart has no bar for grow in from the axis.
  for (let i = lastRace(m).order.length; i < CAPSULES.rows.length; i++) growCapsule(ctx, i, lt);
  ctx.restore();
}

/** Capsules grow in over this long each, a 64th apart, the last home on REST. */
export const GROW_DUR = b(0.5);
export const GROW_EACH = b(1 / 16);
export const GROW_AT = REST - GROW_DUR - (CAPSULES.rows.length - 1) * GROW_EACH;

/**
 * Capsule `i` of race|morph growing out of the axis, from nothing to its
 * full length with a slight overshoot, landing exactly on it by REST.
 */
export function growCapsule(ctx: CanvasRenderingContext2D, i: number, lt: number): void {
  const k = land(lt, GROW_AT + i * GROW_EACH, GROW_DUR, 0.06);
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
