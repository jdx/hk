// K3: the file lanes (storyboard §4 K3), used by stash, lanes, restore and
// everywhere. One lane per staged file, a padlock per lane, and the commit's
// seven steps as bars: warm fix bars that take the write locks on their
// files, so steps on different files run at once and steps on the same file
// take turns. SCHEDULE is the pre-commit fix run from canon80's
// commit.frames.txt; `lanes` plays it with drawGantt and `restore` starts
// from drawFinalGantt, which is the lanes|restore handoff.
//
// The order is real and the widths are schematic ("Order from one real
// commit. Not to scale."): a step's end is when hk's header count ticked
// for it, not when its ✔ row appeared, since the ✔ rows wait for `git add`.

import { LANE, PALETTE } from "../bible";
import { rgba } from "../color";
import { roundedRect } from "../fx";
import { clamp, DEG, lerp, progress, swiftOut } from "../math";
import { drawText, font, layout } from "../type";
import { drawMono, monoWidth } from "./card";
import { land } from "./motion";

/** Layout, logical px. Lane centres are `rows`; bars sit 8 px inside the track. */
export const LANES = {
  labelX: 160,
  trackX0: 640,
  trackX1: 1760,
  trackH: 72,
  trackRadius: 10,
  barH: 56,
  barRadius: 8,
  /** Each lane's padlock is centred here. */
  lockX: 600,
  rows: [200, 320, 440, 560],
} as const;

/** The commit's staged files, in lane order. */
export const LANE_FILES = ["README.md", "src/app.ts", "src/main.py", "scripts/deploy.sh"] as const;

/** A value for every lane, or one for all four. */
export type PerLane<T> = T | readonly T[];
const perLane = <T>(v: PerLane<T> | undefined, i: number, dflt: T): T =>
  v === undefined ? dflt : Array.isArray(v) ? ((v as readonly T[])[i] ?? dflt) : (v as T);

/** The label's baseline sits this far below the lane's centre. */
const LABEL_DROP = 14;

export const laneY = (lane: number): number => LANES.rows[lane];
/** The top and bottom of a bar across lanes `first` to `last`. */
export const barSpan = (first: number, last: number): { y0: number; y1: number } => ({
  y0: LANES.rows[first] - LANES.barH / 2,
  y1: LANES.rows[last] + LANES.barH / 2,
});

// Padlocks. A lane's padlock says who holds its file: nobody (open), a
// fixer (write, warm) or checkers (read, cyan, a pip per reader).

export type LockState = "open" | "write" | "read";

export interface PadlockOptions {
  /** How far the shackle is up: 0 shut, 1 open (raised 10 px, turned 20°). By default 1 when open, else 0. */
  lift?: number;
  /** Reader pips above a read lock. */
  pips?: number;
  scale?: number;
  alpha?: number;
}

/**
 * A padlock centred on (cx, cy): a 32×24 body under a shackle of radius 10.
 * Open is an empty text3 outline with the shackle up and turned on its
 * right leg; write is warm, read cyan, both filled.
 */
export function drawPadlock(ctx: CanvasRenderingContext2D, cx: number, cy: number, state: LockState, o: PadlockOptions = {}): void {
  const a = o.alpha ?? 1;
  if (a <= 0) return;
  const lift = o.lift ?? (state === "open" ? 1 : 0);
  const color = state === "write" ? PALETTE.warm : state === "read" ? PALETTE.cyan : PALETTE.text3;
  const fill = state === "write" ? LANE.fix.fill : state === "read" ? LANE.check.fill : null;
  ctx.save();
  ctx.globalAlpha *= a;
  ctx.translate(cx, cy);
  if (o.scale !== undefined) ctx.scale(o.scale, o.scale);
  ctx.strokeStyle = color;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  // The shackle stands on the body's top edge (y -3): legs at x ±10 up to
  // y -9, then the arch. Opening lifts it and swings it on its right leg.
  ctx.save();
  ctx.translate(0, -10 * lift);
  ctx.translate(10, -3);
  ctx.rotate(20 * DEG * lift);
  ctx.translate(-10, 3);
  ctx.lineWidth = 4;
  ctx.beginPath();
  ctx.moveTo(-10, -3);
  ctx.lineTo(-10, -9);
  ctx.arc(0, -9, 10, Math.PI, 0);
  ctx.lineTo(10, -3);
  ctx.stroke();
  ctx.restore();
  roundedRect(ctx, -16, -3, 32, 24, 5);
  if (fill) {
    // Opaque under the tint, so the shackle's feet never show through.
    ctx.fillStyle = PALETTE.bg;
    ctx.fill();
    ctx.fillStyle = fill;
    ctx.fill();
  }
  ctx.lineWidth = 3;
  ctx.stroke();
  const pips = state === "read" ? Math.max(0, Math.round(o.pips ?? 0)) : 0;
  ctx.fillStyle = color;
  for (let i = 0; i < pips; i++) {
    ctx.beginPath();
    ctx.arc((i - (pips - 1) / 2) * 12, -32, 3, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}

// Bars.

export type BarKind = "fix" | "check";

export interface BarRect {
  x0: number;
  x1: number;
  y0: number;
  y1: number;
}

export interface BarOptions {
  label?: string;
  /**
   * The bar's final width, px, when it is still growing: the label is sized
   * for the whole bar and revealed as the bar reaches it, never resized.
   */
  fullWidth?: number;
  /** Set the label up the bar, turned −90° (a bar across 3 or more lanes). */
  rotate?: boolean;
  alpha?: number;
  radius?: number;
  /**
   * Painted under the tint, so a bar across several lanes is one colour over
   * tracks and gaps alike: the track's colour by default.
   */
  base?: string;
}

/** The largest label size that fits `room` px, from `max` down to a floor of 32, never abbreviated. */
function fitSize(ctx: CanvasRenderingContext2D, label: string, room: number, max: number): number {
  const w = layout(ctx, label, font(max, 600)).width;
  return w <= room ? max : Math.max(32, Math.floor((max * room) / w));
}

/**
 * A step's bar: a check (cyan) or a fix (warm) tint with a 2 px outline and
 * its step name in Space Grotesk 600, 40 px from 16 px in, or 36 px up the
 * middle of a tall bar. The label shrinks to fit, to 32 px at the least, and
 * is clipped to the bar.
 */
export function drawBar(ctx: CanvasRenderingContext2D, r: BarRect, kind: BarKind, o: BarOptions = {}): void {
  const a = o.alpha ?? 1;
  const w = r.x1 - r.x0;
  const h = r.y1 - r.y0;
  if (a <= 0 || w <= 0.5 || h <= 0) return;
  const style = kind === "fix" ? LANE.fix : LANE.check;
  ctx.save();
  ctx.globalAlpha *= a;
  roundedRect(ctx, r.x0, r.y0, w, h, o.radius ?? LANES.barRadius);
  ctx.fillStyle = o.base ?? LANE.track;
  ctx.fill();
  ctx.fillStyle = style.fill;
  ctx.fill();
  ctx.strokeStyle = style.stroke;
  ctx.lineWidth = 2;
  ctx.stroke();
  if (o.label) {
    ctx.clip();
    const full = o.fullWidth ?? w;
    if (o.rotate) {
      const size = fitSize(ctx, o.label, h - 32, 36);
      // Rides the middle of the bar as it grows, and comes in once there is room for it.
      const cx = r.x0 + Math.max(w, 0) / 2;
      ctx.globalAlpha *= progress(size * 0.6, size * 1.3, w);
      ctx.translate(cx, (r.y0 + r.y1) / 2);
      ctx.rotate(-Math.PI / 2);
      drawText(ctx, o.label, 0, size * 0.34, { font: font(size, 600), fill: style.text, align: "center" });
    } else {
      const size = fitSize(ctx, o.label, full - 32, 40);
      drawText(ctx, o.label, r.x0 + 16, (r.y0 + r.y1) / 2 + size * 0.34, { font: font(size, 600), fill: style.text });
    }
  }
  ctx.restore();
}

// Waiting chips: a step queued behind a lock, as a dashed outline of the
// bar it will become.

export interface ChipOptions {
  /** Height, px: a bar's 56 on a lane, 40 in the dock. */
  h?: number;
  /** Label size, mono. */
  size?: number;
  /** A small closed warm padlock after the label: the lock it waits for. */
  lock?: boolean;
  alpha?: number;
}

const CHIP_PAD = 16;
const CHIP_LOCK = 0.6;

/** A waiting chip's width for `label`. */
export function chipWidth(label: string, o: ChipOptions = {}): number {
  return CHIP_PAD + monoWidth(label, o.size ?? 32) + (o.lock ? 10 + 32 * CHIP_LOCK + 12 : CHIP_PAD);
}

/** A waiting chip from x, centred on cy: no fill, a 2 px dashed text3 outline, the label in mono text3. Returns its width. */
export function drawWaitingChip(ctx: CanvasRenderingContext2D, x: number, cy: number, label: string, o: ChipOptions = {}): number {
  const w = chipWidth(label, o);
  const a = o.alpha ?? 1;
  if (a <= 0) return w;
  const h = o.h ?? LANES.barH;
  const size = o.size ?? 32;
  ctx.save();
  ctx.globalAlpha *= a;
  roundedRect(ctx, x, cy - h / 2, w, h, LANES.barRadius);
  ctx.setLineDash([...LANE.waiting.dash]);
  ctx.strokeStyle = LANE.waiting.stroke;
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.setLineDash([]);
  drawMono(ctx, label, x + CHIP_PAD, cy + size * 0.3, size, LANE.waiting.text);
  if (o.lock) drawPadlock(ctx, x + w - 12 - 16 * CHIP_LOCK, cy - 1, "write", { scale: CHIP_LOCK });
  ctx.restore();
  return w;
}

/**
 * A done mark: hk's green ✔ as a vector centred on (cx, cy), `size` px like
 * a glyph. The same strokes as term.ts's ✔ (Liberation Mono has none), so
 * the lanes' marks read as the terminal's.
 */
export function drawDone(ctx: CanvasRenderingContext2D, cx: number, cy: number, o: { size?: number; color?: string; alpha?: number; scale?: number } = {}): void {
  const a = o.alpha ?? 1;
  const s = (o.size ?? 40) * (o.scale ?? 1);
  if (a <= 0 || s <= 0) return;
  ctx.save();
  ctx.globalAlpha *= a;
  ctx.strokeStyle = o.color ?? PALETTE.green;
  ctx.lineWidth = 0.13 * s;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  ctx.moveTo(cx - 0.25 * s, cy + 0.03 * s);
  ctx.lineTo(cx - 0.082 * s, cy + 0.3 * s);
  ctx.lineTo(cx + 0.254 * s, cy - 0.3 * s);
  ctx.stroke();
  ctx.restore();
}

// The empty chart.

/** How one lane's padlock is drawn: a state, or a number for an open padlock at that alpha. */
export interface LockDraw {
  state: LockState;
  lift?: number;
  pips?: number;
  alpha?: number;
}

export interface LanesOptions {
  /** Each label's alpha (default 1). */
  labels?: PerLane<number>;
  /** How many characters of each label are typed (default all). */
  chars?: PerLane<number>;
  /** How far each track has drawn on, left to right, 0..1 (default 1). */
  tracks?: PerLane<number>;
  /** Each padlock: a number is an open padlock at that alpha (default 1). */
  locks?: PerLane<number | LockDraw>;
}

/** The lane chart without bars: labels, tracks and padlocks. With no options, the stash|lanes handoff's chart. */
export function drawLanes(ctx: CanvasRenderingContext2D, o: LanesOptions = {}): void {
  LANES.rows.forEach((cy, i) => {
    const la = perLane(o.labels, i, 1);
    const n = perLane(o.chars, i, Infinity);
    if (la > 0 && n > 0) {
      const file = LANE_FILES[i];
      drawMono(ctx, n >= file.length ? file : file.slice(0, Math.floor(n)), LANES.labelX, cy + LABEL_DROP, 40, rgba(PALETTE.text1, la));
    }
    const p = clamp(perLane(o.tracks, i, 1));
    if (p > 0) {
      ctx.save();
      roundedRect(ctx, LANES.trackX0, cy - LANES.trackH / 2, (LANES.trackX1 - LANES.trackX0) * p, LANES.trackH, LANES.trackRadius);
      ctx.fillStyle = LANE.track;
      ctx.fill();
      ctx.restore();
    }
    const lock = perLane<number | LockDraw>(o.locks, i, 1);
    if (typeof lock === "number") drawPadlock(ctx, LANES.lockX, cy, "open", { alpha: lock });
    else drawPadlock(ctx, LANES.lockX, cy, lock.state, lock);
  });
}

// The schedule.

export interface ScheduleStep {
  /** The step's name in hk.pkl, which is its bar's label. */
  step: string;
  /** First and last lane it holds (indices into LANE_FILES). */
  lanes: readonly [first: number, last: number];
  /** Lanes-local beats: it starts, and its job ends. */
  start: number;
  end: number;
  /** Its bar's final extent, px. */
  x0: number;
  x1: number;
  /** The header count its end makes: `done`/7. */
  done: number;
  /** canon80 commit.frames.txt: the frame its `❯ step` row appears, and the frame the header reaches `done`/7. */
  frames: { readonly start: number; readonly end: number };
}

/** px between two bars on one lane where a lock is handed straight on. */
export const HANDOFF_GAP = 8;

/** The pre-commit fix run, in the storyboard's order. Order from the header counts; widths schematic. */
export const SCHEDULE: readonly ScheduleStep[] = [
  { step: "prettier", lanes: [0, 1], start: 1, end: 8, x0: 648, x1: 1320, done: 5, frames: { start: 3, end: 13 } },
  { step: "ruff", lanes: [2, 2], start: 1, end: 3.5, x0: 648, x1: 880, done: 2, frames: { start: 3, end: 5 } },
  { step: "shfmt", lanes: [3, 3], start: 1, end: 2.5, x0: 648, x1: 800, done: 1, frames: { start: 3, end: 4 } },
  { step: "shellcheck", lanes: [3, 3], start: 2.5, end: 5, x0: 808, x1: 1100, done: 3, frames: { start: 4, end: 7 } },
  { step: "ruff-format", lanes: [2, 2], start: 4, end: 6.5, x0: 908, x1: 1230, done: 4, frames: { start: 6, end: 10 } },
  { step: "trailing-whitespace", lanes: [0, 3], start: 8, end: 10, x0: 1328, x1: 1500, done: 6, frames: { start: 13, end: 14 } },
  { step: "newlines", lanes: [0, 3], start: 10, end: 12, x0: 1508, x1: 1690, done: 7, frames: { start: 14, end: 16 } },
];

/** The playhead's keyframes, lanes-local beat → x: bars grow to it. */
export const PLAYHEAD: readonly (readonly [beat: number, x: number])[] = [
  [1, 648],
  [2.5, 800],
  [3.5, 880],
  [4, 908],
  [5, 1100],
  [6.5, 1230],
  [8, 1320],
  [10, 1500],
  [12, 1690],
];

/** The playhead's x at lanes-local `beat`, piecewise linear, held before b1 and after b12. */
export function playheadX(beat: number): number {
  if (beat <= PLAYHEAD[0][0]) return PLAYHEAD[0][1];
  for (let i = 1; i < PLAYHEAD.length; i++) {
    const [b1, x1] = PLAYHEAD[i];
    if (beat <= b1) {
      const [b0, x0] = PLAYHEAD[i - 1];
      return lerp(x0, x1, (beat - b0) / (b1 - b0));
    }
  }
  return PLAYHEAD[PLAYHEAD.length - 1][1];
}

/** The playhead's line: cyan, 2 px, α 0.5, from y 150 to 610. */
export const PLAYHEAD_LINE = { y0: 150, y1: 610, width: 2, alpha: 0.5 } as const;
/** The queue dock above the lanes, where steps that wait on every file sit. */
export const DOCK = { y0: 108, y1: 148 } as const;
/** The dock's chips are 40 px high with 28 px labels. */
const DOCK_SIZE = 28;

/** Waiting chips come in over this half beat, after the handoff frame. */
const CHIPS_IN = 0.5;
/** The playhead appears on the downbeat of the first steps. */
const GO = PLAYHEAD[0][0];
/** A padlock snaps open or shut over a 32nd. */
const SNAP = 1 / 8;
/** A lock handed straight on shows open for a sixteenth before the next step takes it. */
const HANDOFF_OPEN = 1 / 4;
/** The ✔ cascade at the end, one lane per sixteenth. */
const CASCADE = { beat: 12, each: 1 / 4, x: 1730, size: 40 } as const;
/** From here on the chart is still: the lanes|restore handoff. */
export const GANTT_SETTLED = 13;
/** The lanes scene's last beat. */
const GANTT_END = 16;

const lanesOf = (s: ScheduleStep): number[] => Array.from({ length: s.lanes[1] - s.lanes[0] + 1 }, (_, k) => s.lanes[0] + k);
/** Queued steps wait in the dock when they hold more than two lanes, and on their lane otherwise. */
const inDock = (s: ScheduleStep): boolean => s.lanes[1] - s.lanes[0] >= 2;
/** When a step's write locks shut: at its start, or a sixteenth later when it takes a lock straight from another step. */
function lockedFrom(s: ScheduleStep): number {
  const handed = SCHEDULE.some((o) => o !== s && o.end === s.start && lanesOf(o).some((l) => l >= s.lanes[0] && l <= s.lanes[1]));
  return s.start + (handed ? HANDOFF_OPEN : 0);
}

export interface GanttBar {
  step: ScheduleStep;
  /** Its right edge now: grown to the playhead, capped at its end. */
  x1: number;
  /** 0 while running, then the done badge's life 0..1 over the beat after its end. */
  since: number;
}

export interface GanttChip {
  step: ScheduleStep;
  x: number;
  cy: number;
  dock: boolean;
  alpha: number;
}

export interface GanttLock {
  state: LockState;
  lift: number;
}

export interface GanttState {
  playhead: { x: number; alpha: number };
  /** Steps that have started, in SCHEDULE order. */
  bars: GanttBar[];
  /** Steps still queued (and fading as they start). */
  chips: GanttChip[];
  /** One per lane. */
  locks: GanttLock[];
  /** The end cascade's ✔ per lane, 0..1 (landing with a little overshoot). */
  checks: number[];
}

/**
 * Everything the chart shows at lanes-local `beat`, as data: scenes that
 * need to hang something on a bar, a chip or a padlock read it from here.
 */
export function ganttAt(beat: number): GanttState {
  const ph = playheadX(beat);
  const bars = SCHEDULE.filter((s) => beat >= s.start).map((s) => ({
    step: s,
    x1: clamp(ph, s.x0, s.x1),
    since: beat >= s.end ? progress(s.end, s.end + 1, beat) : 0,
  }));
  // Dock chips queue from the right: each at its slot, or just left of the next.
  const docked = chipLayout();
  const chips: GanttChip[] = [];
  for (const s of SCHEDULE) {
    if (s.start <= GO) continue;
    const alpha = progress(CHIPS_IN, CHIPS_IN + 0.25, beat) * (1 - progress(s.start, s.start + 0.25, beat));
    if (alpha <= 0) continue;
    const dock = inDock(s);
    // A chip on a lane rides ahead of the playhead; the dock is above its reach.
    const x = dock ? docked.dockX.get(s.step)! : Math.max(s.x0, beat >= GO ? playheadX(Math.min(beat, s.start)) + 12 : s.x0);
    const settle = swiftOut(progress(CHIPS_IN, CHIPS_IN + 0.5, beat));
    const cy = dock ? (DOCK.y0 + DOCK.y1) / 2 - 24 * (1 - settle) : LANES.rows[s.lanes[0]];
    chips.push({ step: s, x, cy, dock, alpha });
  }
  const locks = LANES.rows.map((_, lane) => lockAt(lane, beat));
  const checks = LANES.rows.map((_, lane) => land(beat, CASCADE.beat + lane * CASCADE.each, 1 / 4, 0.25));
  return {
    playhead: { x: ph, alpha: PLAYHEAD_LINE.alpha * progress(GO, GO + SNAP, beat) * (1 - progress(CASCADE.beat, CASCADE.beat + 1, beat)) },
    bars,
    chips,
    locks,
    checks,
  };
}

// Chip widths come from the mono grid, not from measuring, so the dock's
// layout needs no context and is worked out once.
let dockLayout: { dockX: Map<string, number> } | null = null;
function chipLayout(): { dockX: Map<string, number> } {
  if (dockLayout) return dockLayout;
  const dockX = new Map<string, number>();
  const queued = SCHEDULE.filter((s) => s.start > GO && inDock(s)).sort((a, b) => b.start - a.start);
  let right = Infinity;
  for (const s of queued) {
    const w = chipWidth(s.step, { size: DOCK_SIZE, lock: true });
    const x = Math.min(s.x0, right - HANDOFF_GAP - w);
    dockX.set(s.step, x);
    right = x;
  }
  dockLayout = { dockX };
  return dockLayout;
}

/** Lane `lane`'s padlock at `beat`: shut while a step holds it, snapping over a 32nd. */
function lockAt(lane: number, beat: number): GanttLock {
  const holds = SCHEDULE.filter((s) => lane >= s.lanes[0] && lane <= s.lanes[1]).map((s) => ({ from: lockedFrom(s), to: s.end }));
  const held = holds.find((h) => beat >= h.from && beat < h.to);
  if (held) return { state: "write", lift: 1 - land(beat, held.from, SNAP, 0.2) };
  const released = holds.filter((h) => h.to <= beat).reduce((m, h) => Math.max(m, h.to), -Infinity);
  return { state: "open", lift: released === -Infinity ? 1 : land(beat, released, SNAP, 0.2) };
}

export interface GanttOptions {
  /** Labels, tracks and padlocks (default true). */
  lanes?: boolean;
  playhead?: boolean;
  /** Waiting chips, on the lanes and in the dock. */
  chips?: boolean;
  /** ✔ marks: a badge over each bar's end as it finishes, and the end cascade. */
  done?: boolean;
}

/** One step's bar from its start to `x1`, labelled and sized for its whole extent. */
export function drawScheduleBar(ctx: CanvasRenderingContext2D, s: ScheduleStep, x1 = s.x1, alpha = 1): void {
  const { y0, y1 } = barSpan(s.lanes[0], s.lanes[1]);
  drawBar(ctx, { x0: s.x0, x1, y0, y1 }, "fix", { label: s.step, fullWidth: s.x1 - s.x0, rotate: s.lanes[1] - s.lanes[0] >= 2, alpha });
}

/**
 * The commit's fix run at lanes-local `beat`: bars grown to the playhead,
 * waiting chips riding ahead of it, the padlocks as the steps take and
 * hand on their files, and the ✔s. From GANTT_SETTLED it is still, and it
 * is drawFinalGantt.
 */
export function drawGantt(ctx: CanvasRenderingContext2D, beat: number, o: GanttOptions = {}): void {
  const g = ganttAt(beat);
  if (o.lanes !== false) drawLanes(ctx, { locks: g.locks.map((l) => ({ state: l.state, lift: l.lift })) });
  for (const b of g.bars) drawScheduleBar(ctx, b.step, b.x1);
  if (o.chips !== false) {
    for (const c of g.chips) {
      drawWaitingChip(ctx, c.x, c.cy, c.step.step, c.dock ? { h: DOCK.y1 - DOCK.y0, size: DOCK_SIZE, lock: true, alpha: c.alpha } : { alpha: c.alpha });
    }
  }
  if (o.done !== false) {
    // A badge over the end of each bar that finishes before the cascade, gone a beat later.
    for (const b of g.bars) {
      if (b.since <= 0 || b.since >= 1 || b.step.end >= CASCADE.beat) continue;
      const { y0 } = barSpan(b.step.lanes[0], b.step.lanes[1]);
      const pop = land(b.since, 0, 0.25, 0.3);
      drawDone(ctx, b.step.x1 - 18, y0 - 22, { size: 32, scale: pop, alpha: 1 - progress(0.5, 1, b.since) });
    }
    g.checks.forEach((k, lane) => {
      if (k > 0) drawDone(ctx, CASCADE.x, LANES.rows[lane], { size: CASCADE.size, scale: k });
    });
  }
  if (o.playhead !== false && g.playhead.alpha > 0) {
    ctx.save();
    ctx.fillStyle = rgba(PALETTE.cyan, g.playhead.alpha);
    ctx.fillRect(g.playhead.x - PLAYHEAD_LINE.width / 2, PLAYHEAD_LINE.y0, PLAYHEAD_LINE.width, PLAYHEAD_LINE.y1 - PLAYHEAD_LINE.y0);
    ctx.restore();
  }
}

/** The finished run, the lanes|restore handoff: every bar at its full extent, the padlocks open, a ✔ on each lane at x 1730. */
export function drawFinalGantt(ctx: CanvasRenderingContext2D): void {
  drawGantt(ctx, GANTT_END);
}
