// K3: the file lanes (storyboard §4 K3), used by stash, lanes, restore and
// everywhere. One lane per staged file, a padlock per lane, and the commit's
// seven steps as bars. A step holds its files in phases: a read phase (a
// cyan bar) while a `check_diff` declared `effect = "read"` works out its
// patch under read locks, and a write phase (a warm bar) while a fixer
// runs, while a `check_diff` declared `effect = "write"` (ruff's) works out
// its patch, or while hk applies a patch. Any number of readers can share a
// file; a writer has it to itself. SCHEDULE
// is the pre-commit fix run from the commit capture
// (test/captures/commit.frames.txt); `lanes` plays it with drawGantt and
// `restore` starts from drawFinalGantt, which is the lanes|restore handoff.
//
// The order is real and the widths are schematic ("Order from one real
// commit. Not to scale."): which steps start together, which share a file,
// and the order they finish in all come from the capture, and the order of
// each step's patch and hand-off from the run's log (commit.locks.txt).
// The board draws each step's own holds; the read locks a step takes to
// stage its files, and ruff letting src/main.py go between applying its
// patch and rechecking the file, are left out. The widths follow the run's
// shape: prettier is the long pole the others run alongside, and every
// other step is quick.

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
  /**
   * Each lane's padlock is centred here: 22 px clear of the longest label
   * (`scripts/deploy.sh` ends at x 568) and 18 px short of the track.
   */
  lockX: 606,
  rows: [200, 320, 440, 560],
} as const;

/** The commit's staged files, in lane order. */
export const LANE_FILES = ["README.md", "src/app.ts", "src/main.py", "scripts/deploy.sh"] as const;

/** A value for every lane, or one for all four. */
export type PerLane<T> = T | readonly T[];
const perLane = <T>(v: PerLane<T> | undefined, i: number, dflt: T): T =>
  v === undefined ? dflt : Array.isArray(v) ? ((v as readonly T[])[i] ?? dflt) : (v as T);

/**
 * Opaque bar fills, one per kind: a dark saturated amber under the warm
 * outline and a dark teal under the cyan one, so a fix bar reads warm and a
 * check bar cyan even at thumbnail size (a 0.2 tint over the track reads
 * grey). Padlocks take the same fills, so a write lock is the colour of the
 * bar that holds it.
 */
export const BAR_FILL = {
  /** hsl(33 45% 20%) */
  fix: "#4a351c",
  /** hsl(186 50% 17%) */
  check: "#163d41",
} as const;

/** The label's baseline sits this far below the lane's centre. */
const LABEL_DROP = 14;

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
  /** Reader pips above a read lock. They rise with the shackle, so they clear its arch while it snaps shut. */
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
  const fill = state === "write" ? BAR_FILL.fix : state === "read" ? BAR_FILL.check : null;
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
    // Opaque, so the shackle's feet never show through.
    ctx.fillStyle = fill;
    ctx.fill();
  }
  ctx.lineWidth = 3;
  ctx.stroke();
  const pips = state === "read" ? Math.max(0, Math.round(o.pips ?? 0)) : 0;
  ctx.fillStyle = color;
  for (let i = 0; i < pips; i++) {
    ctx.beginPath();
    ctx.arc((i - (pips - 1) / 2) * 12, -32 - 10 * lift, 3, 0, Math.PI * 2);
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
  /** The fill, opaque: BAR_FILL for its kind by default. */
  fill?: string;
  /** The label's largest size and the floor it shrinks to, px: 40 (36 turned) and 32 by default. */
  size?: number;
  floor?: number;
  /** A turned label's offset from the bar's middle, px: twins set their names side by side. */
  dx?: number;
}

/** The largest label size that fits `room` px, from `max` down to `floor` (32 by default), never abbreviated. */
export function fitSize(ctx: CanvasRenderingContext2D, label: string, room: number, max: number, floor = 32): number {
  const w = layout(ctx, label, font(max, 600)).width;
  return w <= room ? max : Math.max(floor, Math.floor((max * room) / w));
}

/**
 * A step's bar: a check (dark teal) or a fix (dark amber) fill, opaque, so
 * a bar across several lanes is one colour over tracks and gaps alike, with
 * a 2 px cyan or warm outline and
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
  ctx.fillStyle = o.fill ?? BAR_FILL[kind];
  ctx.fill();
  ctx.strokeStyle = style.stroke;
  ctx.lineWidth = 2;
  ctx.stroke();
  if (o.label) {
    ctx.clip();
    drawBarLabel(ctx, r, kind, o.label, o);
  }
  ctx.restore();
}

/**
 * A bar's label exactly as drawBar sets it on `r`, unclipped: up the middle
 * of a tall bar, else 16 px in, sized for `fullWidth`. Either comes in with
 * the bar's width, so a thin bar never shows glyph slivers. A scene that
 * shows a name before its bar has grown draws it here, on the bar's full
 * rect, and it is drawBar's own label to the pixel.
 */
export function drawBarLabel(ctx: CanvasRenderingContext2D, r: BarRect, kind: BarKind, label: string, o: Pick<BarOptions, "fullWidth" | "rotate" | "size" | "floor" | "dx"> & { color?: string } = {}): void {
  const w = r.x1 - r.x0;
  const h = r.y1 - r.y0;
  const fill = o.color ?? (kind === "fix" ? LANE.fix : LANE.check).text;
  ctx.save();
  if (o.rotate) {
    const size = fitSize(ctx, label, h - 32, o.size ?? 36, o.floor);
    // Rides the middle of the bar as it grows, and comes in once there is room for it.
    const cx = r.x0 + Math.max(w, 0) / 2 + (o.dx ?? 0);
    ctx.globalAlpha *= progress(size * 0.6, size * 1.3, w);
    ctx.translate(cx, (r.y0 + r.y1) / 2);
    ctx.rotate(-Math.PI / 2);
    drawText(ctx, label, 0, size * 0.34, { font: font(size, 600), fill, align: "center" });
  } else {
    const size = fitSize(ctx, label, (o.fullWidth ?? w) - 32, o.size ?? 40, o.floor);
    ctx.globalAlpha *= progress(16, 56, w);
    drawText(ctx, label, r.x0 + 16, (r.y0 + r.y1) / 2 + size * 0.34, { font: font(size, 600), fill });
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
  /** The outline's dash offset, px: march it to show the chip waiting. */
  dashOffset?: number;
  /** The outline's colour, text3 by default. */
  stroke?: string;
  /** How far the small padlock's shackle is up (0 shut, the default; 1 open), as its lock is let go. */
  lockLift?: number;
  /** Draw the dashed outline (default true); false draws only the label and the padlock. */
  outline?: boolean;
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
  if (o.outline !== false) {
    roundedRect(ctx, x, cy - h / 2, w, h, LANES.barRadius);
    ctx.setLineDash([...LANE.waiting.dash]);
    ctx.lineDashOffset = o.dashOffset ?? 0;
    ctx.strokeStyle = o.stroke ?? LANE.waiting.stroke;
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.setLineDash([]);
    ctx.lineDashOffset = 0;
  }
  drawMono(ctx, label, x + CHIP_PAD, cy + size * 0.3, size, LANE.waiting.text);
  if (o.lock) {
    const lx = x + w - 12 - 16 * CHIP_LOCK;
    ctx.save();
    ctx.translate(lx, cy - 1);
    drawPadlock(ctx, 0, 0, "write", { scale: CHIP_LOCK, lift: o.lockLift ?? 0 });
    ctx.restore();
  }
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
  /** A punch as it snaps: 1 at rest. */
  scale?: number;
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

/** How a step holds its files: shared with other readers, or alone. */
export type LockMode = "read" | "write";

/** One stretch of a step holding its files, and its bar. */
export interface Phase {
  lock: LockMode;
  /** Lanes-local beats: it takes the lock (see phaseLockedFrom), and lets it go. */
  start: number;
  end: number;
  /** Its bar's final extent, px. */
  x0: number;
  x1: number;
}

export interface ScheduleStep {
  /** The step's name in hk.pkl, which is its bar's label. */
  step: string;
  /** First and last lane it holds (indices into LANE_FILES). */
  lanes: readonly [first: number, last: number];
  /**
   * Its phases, in order. A gap between two is the step waiting, holding
   * nothing: a read-locked `check_diff` lets its read locks go before it
   * asks for the write locks its patch needs.
   */
  phases: readonly Phase[];
  /**
   * A reader that shares its lane with another reader takes half of the
   * lane's band: 0 the top, 1 the bottom. Steps that hold every lane
   * together (`twin`) share the whole band instead.
   */
  half?: 0 | 1;
  /** The other step of a pair that reads the same lanes over the same beats, drawn as one bar with both names. */
  twin?: string;
  /** The step's span: its first phase's start and x0, its last phase's end and x1. */
  start: number;
  end: number;
  x0: number;
  x1: number;
  /** The header count its end makes: `done`/7. */
  done: number;
  /** commit.frames.txt: the frame its `❯ step` row appears, and the frame the header reaches `done`/7. */
  frames: { readonly start: number; readonly end: number };
}

/** px between two bars on one lane where a lock is handed straight on. */
export const HANDOFF_GAP = 8;

/** A step from its phases: its span is theirs. */
function sched(step: string, lanes: readonly [number, number], phases: readonly Phase[], done: number, frames: { start: number; end: number }, o: { half?: 0 | 1; twin?: string } = {}): ScheduleStep {
  const first = phases[0];
  const last = phases[phases.length - 1];
  return { step, lanes, phases, ...o, start: first.start, end: last.end, x0: first.x0, x1: last.x1, done, frames };
}

const read = (start: number, end: number, x0: number, x1: number): Phase => ({ lock: "read", start, end, x0, x1 });
const write = (start: number, end: number, x0: number, x1: number): Phase => ({ lock: "write", start, end, x0, x1 });

/**
 * The pre-commit fix run. prettier and ruff take write locks; shfmt and
 * shellcheck read scripts/deploy.sh together; shfmt, holding its patch,
 * waits for shellcheck to finish reading, then writes; ruff-format, which
 * depends on ruff, reads main.py, then writes its patch; and
 * trailing-whitespace and newlines, waiting on prettier's files, read all
 * four together and find nothing to fix.
 */
export const SCHEDULE: readonly ScheduleStep[] = [
  sched("prettier", [0, 1], [write(1, 10, 648, 1500)], 5, { start: 3, end: 14 }),
  sched("ruff", [2, 2], [write(1, 4.5, 648, 980)], 3, { start: 3, end: 9 }),
  sched("ruff-format", [2, 2], [read(5, 7.5, 1027, 1264), write(7.5, 8.5, 1272, 1358)], 4, { start: 10, end: 12 }),
  sched("shfmt", [3, 3], [read(1, 2, 648, 743), write(3, 4, 846, 932)], 2, { start: 3, end: 6 }, { half: 0 }),
  sched("shellcheck", [3, 3], [read(1, 3, 648, 838)], 1, { start: 3, end: 5 }, { half: 1 }),
  sched("trailing-whitespace", [0, 3], [read(10, 12, 1508, 1690)], 6, { start: 14, end: 17 }, { twin: "newlines" }),
  sched("newlines", [0, 3], [read(10, 12, 1508, 1690)], 7, { start: 14, end: 18 }, { twin: "trailing-whitespace" }),
];

/**
 * The files each step wrote, by lane: the `N files modified` its ✔ row
 * printed in commit.screen.txt. shellcheck, trailing-whitespace and
 * newlines found nothing to change, so they wrote nothing.
 */
export const WROTE: Readonly<Record<string, readonly number[]>> = {
  prettier: [0, 1],
  ruff: [2],
  "ruff-format": [2],
  shfmt: [3],
  shellcheck: [],
  "trailing-whitespace": [],
  newlines: [],
};

/** The playhead's keyframes, lanes-local beat → x: bars grow to it, about 95 px a beat throughout. */
export const PLAYHEAD: readonly (readonly [beat: number, x: number])[] = [
  [1, 648],
  [2, 743],
  [3, 838],
  [4, 932],
  [4.5, 980],
  [5, 1027],
  [7.5, 1264],
  [8.5, 1358],
  [10, 1500],
  [12, 1690],
];

/**
 * The playhead's speed at each keyframe, px per beat: a monotone cubic
 * (PCHIP, Fritsch–Butland weights) through the keyframes, so it never runs
 * backward or overshoots a keyframe and its speed never jumps where two
 * stretches meet. It leaves b1 at the first stretch's speed and comes to
 * rest on b12.
 */
const PLAYHEAD_SPEED: readonly number[] = (() => {
  const n = PLAYHEAD.length;
  const h = PLAYHEAD.slice(1).map(([b, _], i) => b - PLAYHEAD[i][0]);
  const d = PLAYHEAD.slice(1).map(([_, x], i) => (x - PLAYHEAD[i][1]) / h[i]);
  const m = new Array<number>(n).fill(0);
  m[0] = d[0];
  for (let k = 1; k < n - 1; k++) {
    if (d[k - 1] * d[k] <= 0) continue;
    const w1 = 2 * h[k] + h[k - 1];
    const w2 = h[k] + 2 * h[k - 1];
    m[k] = (w1 + w2) / (w1 / d[k - 1] + w2 / d[k]);
  }
  m[n - 1] = 0;
  return m;
})();

/**
 * The playhead's x at lanes-local `beat`, held before b1 and after b12: a
 * smooth curve through the keyframes, exactly on each keyframe's x on its
 * beat, so every bar starts and ends on the playhead.
 */
export function playheadX(beat: number): number {
  if (beat <= PLAYHEAD[0][0]) return PLAYHEAD[0][1];
  for (let i = 1; i < PLAYHEAD.length; i++) {
    const [b1, x1] = PLAYHEAD[i];
    if (beat < b1) {
      const [b0, x0] = PLAYHEAD[i - 1];
      const h = b1 - b0;
      const t = (beat - b0) / h;
      const t2 = t * t;
      const t3 = t2 * t;
      return (2 * t3 - 3 * t2 + 1) * x0 + (t3 - 2 * t2 + t) * h * PLAYHEAD_SPEED[i - 1] + (3 * t2 - 2 * t3) * x1 + (t3 - t2) * h * PLAYHEAD_SPEED[i];
    }
    if (beat === b1) return x1;
  }
  return PLAYHEAD[PLAYHEAD.length - 1][1];
}

/** The playhead's line: cyan, 2 px, α 0.5, from y 150 to 610. */
export const PLAYHEAD_LINE = { y0: 150, y1: 610, width: 2, alpha: 0.5 } as const;
/** The queue dock above the lanes, where steps that wait on every file sit. */
export const DOCK = { y0: 108, y1: 148 } as const;
/** The dock's chips are 40 px high with 28 px labels. */
const DOCK_SIZE = 28;
/** How a dock chip is drawn: 40 px high, a 28 px label and the lock it waits for. */
export const DOCK_CHIP = { h: DOCK.y1 - DOCK.y0, size: DOCK_SIZE, lock: true } as const;

/** Waiting chips come in over this half beat, after the handoff frame. */
const CHIPS_IN = 0.5;
/** The playhead appears on the downbeat of the first steps. */
const GO = PLAYHEAD[0][0];
/** A padlock snaps open or shut over a 32nd. */
const SNAP = 1 / 8;
/** A lock handed straight on shows open for a sixteenth before the next step takes it. */
const HANDOFF_OPEN = 1 / 4;
/** The ✔ cascade at the end, one lane per sixteenth. */
export const CASCADE = { beat: 12, each: 1 / 4, x: 1730, size: 40 } as const;
/** From here on the chart is still: the lanes|restore handoff. */
export const GANTT_SETTLED = 13;
/** The lanes scene's last beat. */
const GANTT_END = 16;

/** The lanes a step holds, first to last. */
export const lanesOf = (s: ScheduleStep): number[] => Array.from({ length: s.lanes[1] - s.lanes[0] + 1 }, (_, k) => s.lanes[0] + k);
/** Queued steps wait in the dock when they hold more than two lanes, and on their lane otherwise. */
export const inDock = (s: ScheduleStep): boolean => s.lanes[1] - s.lanes[0] >= 2;
const sharesLane = (a: ScheduleStep, b: ScheduleStep): boolean => a.lanes[0] <= b.lanes[1] && b.lanes[0] <= a.lanes[1];
/** Two phases that can't hold a file at once: a writer shares with nobody. */
const conflicts = (a: Phase, b: Phase): boolean => a.lock === "write" || b.lock === "write";

/**
 * When phase `i` of step `s` takes its lock: at its start, or a sixteenth
 * later when it takes the files straight from a phase it can't share them
 * with, another step's or its own read phase trading up to write.
 */
export function phaseLockedFrom(s: ScheduleStep, i: number): number {
  const p = s.phases[i];
  const handed = SCHEDULE.some((o) => sharesLane(o, s) && o.phases.some((q, j) => !(o === s && j === i) && q.end === p.start && conflicts(p, q)));
  return p.start + (handed ? HANDOFF_OPEN : 0);
}

/** When a step first takes its lock: its first phase's. */
export const lockedFrom = (s: ScheduleStep): number => phaseLockedFrom(s, 0);

/**
 * How far step `s`'s bars have grown at `beat`: to the playhead through each
 * phase whose lock it holds, along the dashed tie while it waits for the
 * next, and up to that phase's start until its lock shuts. A phase handed a
 * lock a sixteenth after the playhead passes its start catches up over a
 * 32nd, rather than appearing part-grown. s.x0 before its first lock.
 */
export function stepReach(s: ScheduleStep, beat: number): number {
  const ph = playheadX(beat);
  let reach = s.x0;
  for (let i = 0; i < s.phases.length; i++) {
    const p = s.phases[i];
    const from = phaseLockedFrom(s, i);
    if (beat < from) {
      if (i > 0) reach = Math.max(reach, Math.min(ph, p.x0));
      break;
    }
    let r = clamp(ph, p.x0, p.x1);
    if (from > p.start) r = lerp(p.x0, r, swiftOut(progress(from, from + 1 / 8, beat)));
    reach = Math.max(reach, r);
  }
  return reach;
}

/** Every phase that holds lane `lane`, from when its lock is taken to when it is let go. */
export interface Hold {
  step: ScheduleStep;
  lock: LockMode;
  from: number;
  to: number;
}
export const holdsOn = (lane: number): Hold[] =>
  SCHEDULE.filter((s) => lane >= s.lanes[0] && lane <= s.lanes[1]).flatMap((s) => s.phases.map((p, i) => ({ step: s, lock: p.lock, from: phaseLockedFrom(s, i), to: p.end })));

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
  /** Readers holding a read lock; 0 otherwise. */
  pips: number;
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
  const bars = SCHEDULE.filter((s) => beat >= lockedFrom(s)).map((s) => ({
    step: s,
    x1: stepReach(s, beat),
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
  // The last to start sits nearest its slot, inside the stage's right
  // margin; of two that start together, the later in SCHEDULE, so twins
  // wait in the order their names stand on their bar.
  const queued = SCHEDULE.filter((s) => s.start > GO && inDock(s)).sort((a, b) => b.start - a.start || SCHEDULE.indexOf(b) - SCHEDULE.indexOf(a));
  let right = LANES.trackX1 + HANDOFF_GAP;
  for (const s of queued) {
    const w = chipWidth(s.step, { size: DOCK_SIZE, lock: true });
    const x = Math.min(s.x0, right - HANDOFF_GAP - w);
    dockX.set(s.step, x);
    right = x;
  }
  dockLayout = { dockX };
  return dockLayout;
}

/** A change in a lane's padlock: it shuts (for a writer or for readers), it opens, or a reader joins or leaves. */
export interface LockChange {
  at: number;
  change: "shut" | "open" | "pips";
  state: LockState;
  /** Readers holding it after the change. */
  pips: number;
}

/** Every change in lane `lane`'s padlock, in order: what the picture punches and the score clicks. */
export function lockChanges(lane: number): LockChange[] {
  const holds = holdsOn(lane);
  const times = [...new Set(holds.flatMap((h) => [h.from, h.to]))].sort((a, b) => a - b);
  const out: LockChange[] = [];
  let prev: { state: LockState; pips: number } = { state: "open", pips: 0 };
  for (const at of times) {
    const now = holds.filter((h) => at >= h.from && at < h.to);
    const state: LockState = now.length === 0 ? "open" : now.some((h) => h.lock === "write") ? "write" : "read";
    const pips = state === "read" ? now.length : 0;
    if (state === prev.state && pips === prev.pips) continue;
    const change = state === "open" ? "open" : prev.state === "open" || state !== prev.state ? "shut" : "pips";
    out.push({ at, change, state, pips });
    prev = { state, pips };
  }
  return out;
}

/**
 * Lane `lane`'s padlock at `beat`: warm while a writer holds the file, cyan
 * with a pip per reader while readers do, open otherwise. The shackle snaps
 * over a 32nd when the file goes from free to held and back; a reader
 * joining or leaving only changes the pips.
 */
function lockAt(lane: number, beat: number): GanttLock {
  const holds = holdsOn(lane);
  const now = holds.filter((h) => beat >= h.from && beat < h.to);
  // The stretch of held time the lane is in, or the last one before it.
  const spans = holds.map((h) => [h.from, h.to] as [number, number]).sort((a, b) => a[0] - b[0]);
  const merged: [number, number][] = [];
  for (const [from, to] of spans) {
    const last = merged[merged.length - 1];
    if (last && from <= last[1]) last[1] = Math.max(last[1], to);
    else merged.push([from, to]);
  }
  const span = merged.find(([from, to]) => beat >= from && beat < to);
  if (span) {
    const lift = 1 - land(beat, span[0], SNAP, 0.2);
    if (now.some((h) => h.lock === "write")) return { state: "write", lift, pips: 0 };
    return { state: "read", lift, pips: now.length };
  }
  const released = merged.filter(([, to]) => to <= beat).reduce((m, [, to]) => Math.max(m, to), -Infinity);
  return { state: "open", lift: released === -Infinity ? 1 : land(beat, released, SNAP, 0.2), pips: 0 };
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

/** A half of a lane's band, for two readers sharing it: 26 px each, 4 px apart, filling the bar's 56. */
export const HALF = { h: 26, gap: 4, size: 22 } as const;

/** The top and bottom of step `s`'s bars: its lanes' band, or the half of it a shared reader takes. */
export function stepSpan(s: ScheduleStep): { y0: number; y1: number } {
  const { y0, y1 } = barSpan(s.lanes[0], s.lanes[1]);
  if (s.half === undefined) return { y0, y1 };
  const cy = (y0 + y1) / 2;
  return s.half === 0 ? { y0: cy - HALF.gap / 2 - HALF.h, y1: cy - HALF.gap / 2 } : { y0: cy + HALF.gap / 2, y1: cy + HALF.gap / 2 + HALF.h };
}

/** A twin's place in its pair: 0 for the first in SCHEDULE, 1 for the second; -1 for a step that is no twin. */
export function twinIndex(s: ScheduleStep): number {
  if (!s.twin) return -1;
  return SCHEDULE.findIndex((o) => o.step === s.step) < SCHEDULE.findIndex((o) => o.step === s.twin) ? 0 : 1;
}

/** How far a twin's name sits from the middle of their shared bar, px. */
export const TWIN_LABEL_DX = 22;

/** A twin's name's offset from the middle of their bar: left for the first, right for the second, 0 for a step that is no twin. */
export const twinDx = (s: ScheduleStep): number => (s.twin ? (twinIndex(s) - 0.5) * 2 * TWIN_LABEL_DX : 0);

/** A phase's bar kind: cyan for a read, warm for a write. */
export const phaseKind = (p: Phase): BarKind => (p.lock === "read" ? "check" : "fix");

/**
 * One step's bars from its start to `x1`: each phase the playhead has
 * reached, a dashed tie where it waits between two, and its name on the
 * first, sized for that phase's whole extent. The last phase runs to `x1`
 * if `x1` is past its end (restore's wind-up). A twin draws its pair's bar
 * once (the first) and each draws its own name up it.
 */
export function drawScheduleBar(ctx: CanvasRenderingContext2D, s: ScheduleStep, x1 = s.x1, alpha = 1, o: { labels?: boolean } = {}): void {
  const { y0, y1 } = stepSpan(s);
  const rotate = s.lanes[1] - s.lanes[0] >= 2;
  const twin = twinIndex(s);
  const labelOpts = s.half === undefined ? {} : { size: HALF.size, floor: 18 };
  s.phases.forEach((p, i) => {
    const last = i === s.phases.length - 1;
    const right = last ? x1 : Math.min(x1, p.x1);
    if (right <= p.x0) return;
    const first = i === 0 && o.labels !== false;
    if (twin !== 1) drawBar(ctx, { x0: p.x0, x1: right, y0, y1 }, phaseKind(p), first && twin < 0 ? { label: s.step, fullWidth: p.x1 - p.x0, rotate, alpha, ...labelOpts } : { alpha });
    if (first && twin >= 0) {
      ctx.save();
      ctx.globalAlpha *= alpha;
      roundedRect(ctx, p.x0, y0, right - p.x0, y1 - y0, LANES.barRadius);
      ctx.clip();
      drawBarLabel(ctx, { x0: p.x0, x1: right, y0, y1 }, phaseKind(p), s.step, { rotate, dx: twinDx(s) });
      ctx.restore();
    }
    // Waiting between two phases: a dashed tie along the step's row.
    const next = s.phases[i + 1];
    if (next && x1 > p.x1 && next.x0 - p.x1 > HANDOFF_GAP) {
      const cy = (y0 + y1) / 2;
      ctx.save();
      ctx.globalAlpha *= alpha;
      ctx.setLineDash([...LANE.waiting.dash]);
      ctx.strokeStyle = LANE.waiting.stroke;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(p.x1 + 4, cy);
      ctx.lineTo(Math.min(x1, next.x0) - 4, cy);
      ctx.stroke();
      ctx.restore();
    }
  });
}

/**
 * The commit's fix run at lanes-local `beat`: bars grown to the playhead,
 * waiting chips riding ahead of it, the padlocks as the steps take and
 * hand on their files, and the ✔s. From GANTT_SETTLED it is still, and it
 * is drawFinalGantt.
 */
export function drawGantt(ctx: CanvasRenderingContext2D, beat: number, o: GanttOptions = {}): void {
  const g = ganttAt(beat);
  if (o.lanes !== false) drawLanes(ctx, { locks: g.locks.map((l) => ({ state: l.state, lift: l.lift, pips: l.pips })) });
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
      const pop = land(b.since, 0, 0.25, 0.3);
      drawDone(ctx, b.step.x1 - 18, badgeY(b.step) - (b.step.half === 1 ? -22 : 22), { size: 32, scale: pop, alpha: 1 - progress(0.5, 1, b.since) });
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

/** Where a finished step's ✔ badge sits: over its bar's top edge, or under the bottom one for the lower of two shared readers. */
export const badgeY = (s: ScheduleStep): number => (s.half === 1 ? stepSpan(s).y1 : stepSpan(s).y0);

/** The finished run, the lanes|restore handoff: every bar at its full extent, the padlocks open, a ✔ on each lane at x 1730. */
export function drawFinalGantt(ctx: CanvasRenderingContext2D): void {
  drawGantt(ctx, GANTT_END);
}
