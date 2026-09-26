// Section 5, "Steps in parallel" (storyboard §6.5): the heart of the reel.
// A commit is a fix run, so every step takes a write lock on each file it
// touches. The pre-commit run of canon80 plays on the four file lanes from
// kit/lanes.ts SCHEDULE: prettier, ruff and shfmt start together on
// different files; shellcheck takes deploy.sh from shfmt; ruff-format waits
// for ruff because it depends on it; and trailing-whitespace and newlines,
// which touch every file, wait in the dock above the lanes and clamp down
// across all four in turn.
//
// What the viewer reads, beat by beat (lanes-local beats, BEATS in lanes-timing.ts):
//
//   b0.5   the queue: the two all-file steps slide into the dock, the
//          waiting chips pop onto lanes 3 and 4, and a dashed bracket ties
//          ruff-format to lane 3's start: depends = "ruff"
//   b1     go: the playhead drops in at x 648, the first three bars grow
//          from it, and all four padlocks snap shut
//   b2.5   shfmt ✔; deploy.sh's padlock opens for a sixteenth and sends
//          a warm key under lane 4 to shellcheck's waiting chip, and shuts
//          as it lands: the chip flushes warm and turns into the bar
//   b3.5   ruff ✔ sends a green spark along the bracket, reading its label
//          through; b4 it snaps taut and fades, and ruff-format starts
//   b5     shellcheck ✔; b6.5 ruff-format ✔; only prettier still runs and
//          the docked steps knock on their locks
//   b8     the clamp: prettier ✔, README.md and src/app.ts flash warm,
//          and trailing-whitespace, wound up over the sixteenth before,
//          springs its small padlock and dives out of the dock; it slams
//          down across all four lanes on b8.25 as every padlock shuts: a
//          dashed outline of the whole bar, its name up the middle, which
//          the bar then grows into
//   b10    trailing-whitespace ✔, and newlines does the same on b10.25
//   b12    newlines ✔, a green ✔ cascade down x 1730, padlocks open
//   b14.5  the detail wipes; from b15 the frame is the lanes|restore
//          handoff (the finished Gantt and the closed tray)
//
// Running bars glow at the playhead and carry a sheen toward it; a finished
// bar flashes, thunks a few px past its end and settles, and wears a ✔
// badge on its corner until the cascade gathers them at b12. Waiting chips
// march; the docked ones knock on the locks they wait for (b6.5–9.5), and
// the lanes holding those locks answer. The slams jolt the chart, the dock
// a little more and the tray a little less, and a light crosses the
// finished chart in the hold. Every frame is a pure function of the beat:
// the chart's state comes from ganttAt(), and everything drawn over it is
// timed from SCHEDULE and BEATS.

import { BEAT, LANE, PALETTE, type Scene, type SceneEnv, sec } from "../bible";
import { mix, rgba } from "../color";
import { glow, ring, roundedRect } from "../fx";
import { drawHandoff } from "../handoff";
import { drawMono, monoWidth } from "../kit/card";
import {
  barSpan,
  CASCADE,
  chipWidth,
  DOCK,
  DOCK_CHIP,
  drawBar,
  drawBarLabel,
  drawDone,
  drawLanes,
  drawScheduleBar,
  drawWaitingChip,
  fitSize,
  ganttAt,
  inDock,
  LANE_FILES,
  LANES,
  lanesOf,
  lockedFrom,
  PLAYHEAD_LINE,
  playheadX,
  SCHEDULE,
  type ScheduleStep,
} from "../kit/lanes";
import { type Curve, drawSpark, jolt, land } from "../kit/motion";
import { drawTray } from "../kit/tray";
import { clamp, inOutSine, inQuad, lerp, progress, smoothstep, swiftOut, TAU } from "../math";
import { type Caption, drawWords, font, layout, wordStyle } from "../type";
import { BEATS } from "./lanes-timing";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 6 words: need 4, hold 4.
  { out: 6, lines: [{ in: 2, text: "Different files? Steps run at once." }] },
  // 5 words: need 3.5, hold 3.5; lands on the clamp's echo. "They", not
  // "Fixes": everywhere's thesis says "Fixes take turns." and this must not
  // pre-empt it word for word.
  { out: 12, lines: [{ in: 8.5, text: "Same file? They take turns." }] },
];

const S = sec("lanes");

/** The picture's beats (lanes-timing.ts, which the score reads on its own). */
export { BEATS };

/** From here the frame is exactly the lanes|restore handoff. */
const SETTLED = 15;

const STEP = Object.fromEntries(SCHEDULE.map((s) => [s.step, s])) as Record<string, ScheduleStep>;
/** A docked step's dive lasts until its locks shut: it lands with them. */
const landOf = (s: ScheduleStep): number => (inDock(s) ? lockedFrom(s) : s.start);

// Small timing shapes, in beats.

/** A flare: 1 on `at`, decaying with `tau`, exactly 0 from at + 6 tau. */
function flare(b: number, at: number, tau: number): number {
  if (b < at) return 0;
  return Math.exp(-(b - at) / tau) * (1 - smoothstep(at + 3 * tau, at + 6 * tau, b));
}

/** Soft light along a line: glow sprites strung from (x0, y0) to (x1, y1). */
function glowLine(ctx: CanvasRenderingContext2D, x0: number, y0: number, x1: number, y1: number, color: string, alpha: number, radius = 34): void {
  if (alpha <= 0) return;
  const n = Math.max(1, Math.round(Math.hypot(x1 - x0, y1 - y0) / (radius * 1.1)));
  for (let i = 0; i < n; i++) glow(ctx, lerp(x0, x1, (i + 0.5) / n), lerp(y0, y1, (i + 0.5) / n), radius, color, alpha);
}

/** Light on a vertical run, from y0 to y1. */
const edgeGlow = (ctx: CanvasRenderingContext2D, x: number, y0: number, y1: number, color: string, alpha: number, radius = 34): void =>
  glowLine(ctx, x, y0, x, y1, color, alpha, radius);

/** A bar's rect as drawScheduleBar draws it, to `x1`. */
function barRect(s: ScheduleStep, x1: number): { x0: number; x1: number; y0: number; y1: number } {
  const { y0, y1 } = barSpan(s.lanes[0], s.lanes[1]);
  return { x0: s.x0, x1, y0, y1 };
}

// The padlocks: each shut and each opening gets a punch, and a shut a
// warm flare. The events are the kit's own lock times.

interface LockEvent {
  at: number;
  shut: boolean;
}
const LOCK_EVENTS: readonly LockEvent[][] = LANES.rows.map((_, lane) =>
  SCHEDULE.filter((s) => lanesOf(s).includes(lane))
    .flatMap((s) => [
      { at: lockedFrom(s), shut: true },
      { at: s.end, shut: false },
    ])
    .sort((a, b) => a.at - b.at),
);
const lastLockEvent = (lane: number, b: number): LockEvent | null =>
  LOCK_EVENTS[lane].reduce<LockEvent | null>((m, e) => (e.at <= b ? e : m), null);

/**
 * A docked step knocking on the locks it waits for: the lanes held at that
 * moment answer with a flare and a small punch (every knock falls while
 * some other step holds the lane, never on a lock event).
 */
function knockOn(b: number, locked: boolean): { flare: number; punch: number } {
  let f = 0;
  let p = 0;
  if (!locked) return { flare: 0, punch: 0 };
  for (const at of BEATS.knocks) {
    if (b < at || b > at + 1) continue;
    f = Math.max(f, flare(b, at, 0.12));
    p += jolt(b, at, 0.375, 3);
  }
  return { flare: f, punch: p };
}

/** The padlock's scale: a punch that overshoots and settles, exactly 1 half a beat on. */
function lockScale(lane: number, b: number, locked: boolean): number {
  const e = lastLockEvent(lane, b);
  const knock = 0.12 * knockOn(b, locked).punch;
  if (!e) return 1 + knock;
  return 1 + (e.shut ? 0.34 : 0.16) * jolt(b, e.at, 0.5, 3) + knock;
}

// The dock: where the two all-file steps wait. Their slots are the kit's.

const DOCKED = SCHEDULE.filter((s) => inDock(s) && s.start > BEATS.go);
const DOCK_X = new Map(ganttAt(2).chips.filter((c) => c.dock).map((c) => [c.step.step, c.x]));
const DOCK_CY = (DOCK.y0 + DOCK.y1) / 2;
/**
 * They slide in from the right as a train, trailing-whitespace first and
 * newlines a 32nd behind it, so the one behind is never over the other.
 */
const dockOrder = (s: ScheduleStep): number => DOCKED.indexOf(s);

/**
 * A knock's shove: up to about 0.72 in the first 32nd of a beat and back to
 * exactly 0 by `at + dur`, with no bounce, so it reads as one hit.
 */
function shove(b: number, at: number, dur: number): number {
  const p = progress(at, at + dur, b);
  if (p <= 0 || p >= 1) return 0;
  return Math.sin((Math.PI / 2) * Math.min(1, p / 0.15)) * (1 - p) ** 2;
}

/** How a docked chip sits at `b`: its slide in, idle bob, knocks on its lock and the wind-up before its dive. */
function dockPose(s: ScheduleStep, b: number): { dx: number; dy: number; sx: number; sy: number; lockAngle: number; flush: number; alpha: number } {
  const k = dockOrder(s);
  const inAt = BEATS.queue + k / 8;
  const alpha = progress(inAt, inAt + 0.25, b);
  const dx = 96 * (1 - swiftOut(progress(inAt, inAt + 0.625, b)));
  // Waiting: a slow bob, the queue breathing together.
  let dy = 2 * Math.sin(TAU * (b / 4)) * smoothstep(1, 2, b);
  // Knocks: it shoves down toward the lanes it wants, the lock rattles and
  // the outline flushes warm.
  let lockAngle = 0;
  let flush = 0;
  for (const at of BEATS.knocks) {
    if (at >= s.start) continue;
    dy += 8 * shove(b, at, 0.375);
    lockAngle += 0.4 * jolt(b, at, 0.45, 5);
    flush = Math.max(flush, flare(b, at, 0.15));
  }
  // The wind-up: it rises and squashes over the sixteenth before it dives.
  const wind = smoothstep(s.start - 0.25, s.start, b);
  dy -= 8 * wind;
  return { dx, dy, sx: 1 + 0.04 * wind, sy: 1 - 0.12 * wind, lockAngle, flush, alpha };
}

/** A docked chip's outline colour: text3, flushed warm as it knocks. */
const dockStroke = (flush: number): string => mix(PALETTE.text3, PALETTE.warmBright, flush);

/** The docked chip's rect in its pose. */
function dockRect(s: ScheduleStep, b: number): { x0: number; x1: number; y0: number; y1: number } {
  const p = dockPose(s, b);
  const w = chipWidth(s.step, DOCK_CHIP);
  const cx = DOCK_X.get(s.step)! + w / 2 + p.dx;
  const cy = DOCK_CY + p.dy;
  return { x0: cx - (w * p.sx) / 2, x1: cx + (w * p.sx) / 2, y0: cy - (DOCK_CHIP.h * p.sy) / 2, y1: cy + (DOCK_CHIP.h * p.sy) / 2 };
}

function drawDockChip(ctx: CanvasRenderingContext2D, s: ScheduleStep, b: number, lt: number): void {
  const p = dockPose(s, b);
  if (p.alpha <= 0) return;
  const w = chipWidth(s.step, DOCK_CHIP);
  const x = DOCK_X.get(s.step)!;
  // The outline is stroked on the posed rect, unscaled, exactly as the dive
  // strokes it, so its dashes carry straight on when the chip leaves.
  const r = dockRect(s, b);
  ctx.save();
  ctx.globalAlpha *= p.alpha;
  roundedRect(ctx, r.x0, r.y0, r.x1 - r.x0, r.y1 - r.y0, LANES.barRadius);
  ctx.setLineDash([...LANE.waiting.dash]);
  ctx.lineDashOffset = -lt * 36;
  ctx.strokeStyle = dockStroke(p.flush);
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.restore();
  ctx.save();
  ctx.translate(x + w / 2 + p.dx, DOCK_CY + p.dy);
  ctx.scale(p.sx, p.sy);
  if (p.flush > 0) {
    // The knock shows at phone size: the lock flares, and light spills off
    // the chip's foot toward the lanes it wants.
    glow(ctx, w / 2 - 22, 0, 56, PALETTE.warm, 0.9 * p.flush);
    glowLine(ctx, -w / 2 + 30, DOCK_CHIP.h / 2, w / 2 - 30, DOCK_CHIP.h / 2, PALETTE.warm, 0.3 * p.flush, 30);
  }
  drawWaitingChip(ctx, -w / 2, 0, s.step, { ...DOCK_CHIP, alpha: p.alpha, outline: false, lockAngle: p.lockAngle });
  ctx.restore();
}

/** How far the docked step's name, falling, trails the blind's foot: trailing-whitespace's gap at rest. */
const GHOST_NAME_LEAD = 38;
/**
 * The two names cross-fade on the dive: the chip's, riding the blind's top
 * corner, gives way (GHOST_CHIP_OUT) as the bar's name, already inside the
 * blind behind its foot, comes up (GHOST_NAME_IN), so the blind is never
 * empty.
 */
const GHOST_CHIP_OUT = (u: number): number => 1 - smoothstep(0.1, 0.4, u);
const GHOST_NAME_IN = (u: number): number => smoothstep(0.2, 0.5, u);

/** The dive: a sixteenth, from the docked step's start to its landing. */
const diveOf = (s: ScheduleStep, b: number): number => progress(s.start, landOf(s), b);

/** The docked chip's rect and pose as it leaves the dock, and the rect it falls into. */
function diveRects(s: ScheduleStep): { from: { x0: number; x1: number; y0: number; y1: number }; full: { x0: number; x1: number; y0: number; y1: number } } {
  return { from: dockRect(s, s.start), full: barRect(s, s.x1) };
}

/** Where the falling blind is at dive progress `u`: it narrows fast and drops accelerating. */
function ghostRect(s: ScheduleStep, u: number): { x0: number; x1: number; y0: number; y1: number } {
  const { from, full } = diveRects(s);
  if (u >= 1) return full;
  const h = swiftOut(u);
  const v = inQuad(u);
  return { x0: lerp(from.x0, full.x0, h), x1: lerp(from.x1, full.x1, h), y0: lerp(from.y0, full.y0, v), y1: lerp(from.y1, full.y1, v) };
}

/**
 * A docked step's dive and its reservation. At its start the wound-up chip
 * slides over its slot, narrowing to the bar's full extent, and falls like
 * a blind across all four lanes, landing as every padlock shuts: a dashed
 * outline of the whole bar, the files it holds. The bar then grows inside
 * it with the playhead, and the outline is gone once the bar fills it.
 */
function drawGhost(ctx: CanvasRenderingContext2D, s: ScheduleStep, b: number, lt: number): void {
  const at = s.start;
  const to = landOf(s);
  if (b < at || b >= s.end + 0.25) return;
  const u = diveOf(s, b);
  const { from, full } = diveRects(s);
  const r = ghostRect(s, u);
  const w = r.x1 - r.x0;
  const slam = flare(b, to, 0.2);
  const fade = 1 - progress(s.end, s.end + 0.25, b);
  const pose = dockPose(s, b);
  ctx.save();
  ctx.globalAlpha *= fade;
  // Falling light at the blind's foot, and the impact along it.
  if (u < 1) glow(ctx, (r.x0 + r.x1) / 2, r.y1, 50, PALETTE.warm, 0.55 * smoothstep(0.3, 0.9, u));
  roundedRect(ctx, r.x0, r.y0, w, r.y1 - r.y0, LANES.barRadius);
  // The chip has no fill; the reservation takes a faint one as it falls.
  ctx.fillStyle = rgba(PALETTE.warm, 0.05 * smoothstep(0, 0.6, u) + 0.1 * slam);
  ctx.fill();
  ctx.setLineDash([...LANE.waiting.dash]);
  ctx.lineDashOffset = -lt * 36;
  ctx.strokeStyle = mix(dockStroke(pose.flush), PALETTE.warmBright, Math.max(0.5 * smoothstep(0, 1, u) * (1 - progress(to, to + 1, b)), slam));
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.setLineDash([]);
  // The chip's name and its small padlock ride the blind's top right corner
  // in the chip's wound-up squash, so the first frame of the dive is the
  // last frame in the dock. The lock it waited for is free: the shackle
  // springs open, and both fade as the blind drops.
  const ca = GHOST_CHIP_OUT(u);
  if (ca > 0) {
    ctx.save();
    ctx.clip();
    const cw = chipWidth(s.step, DOCK_CHIP);
    const sx = (from.x1 - from.x0) / cw;
    const sy = (from.y1 - from.y0) / DOCK_CHIP.h;
    ctx.translate(r.x1 - (cw * sx) / 2, r.y0 + (DOCK_CHIP.h * sy) / 2);
    ctx.scale(sx, sy);
    drawWaitingChip(ctx, -cw / 2, 0, s.step, {
      ...DOCK_CHIP,
      alpha: ca,
      outline: false,
      lockAngle: pose.lockAngle,
      lockLift: land(b, at, 0.0625, 0.3),
    });
    ctx.restore();
  }
  ctx.restore();
  // The impact runs along its foot.
  if (slam > 0) glowLine(ctx, full.x0, full.y1, full.x1, full.y1, PALETTE.warm, 0.35 * slam * fade, 30);
}

/**
 * The docked step's name, set up the middle of the bar it reserves: drawBar's
 * own rotated label. It falls just behind the blind's foot, first letter
 * leading, and comes to rest exactly where the finished bar will carry it; it stays
 * put while the bar grows in under it, and the bar's label takes over, to
 * the pixel, when the bar is whole. One name is on screen from the dock to
 * the end.
 */
function drawGhostLabel(ctx: CanvasRenderingContext2D, s: ScheduleStep, b: number): void {
  if (b < s.start || b >= s.end) return;
  const u = diveOf(s, b);
  const a = GHOST_NAME_IN(u);
  if (a <= 0) return;
  const r = ghostRect(s, u);
  const full = barRect(s, s.x1);
  // Its first letter (the bottom of the turned name) rides a fixed gap
  // behind the foot, whatever the name's length, so a short name is inside
  // the blind as soon as a long one; it stops at rest, where the bar will
  // carry it, which a short name reaches just before the blind lands.
  const size = fitSize(ctx, s.step, full.y1 - full.y0 - 32, 36);
  const restLead = (full.y0 + full.y1) / 2 + layout(ctx, s.step, font(size, 600)).width / 2;
  const lead = Math.min(GHOST_NAME_LEAD, full.y1 - restLead);
  const drop = Math.min(0, r.y1 - lead - restLead);
  ctx.save();
  ctx.globalAlpha *= a;
  roundedRect(ctx, r.x0, r.y0, r.x1 - r.x0, r.y1 - r.y0, LANES.barRadius);
  ctx.clip();
  drawBarLabel(ctx, { x0: r.x0, x1: r.x1, y0: full.y0 + drop, y1: full.y1 + drop }, "fix", s.step, {
    rotate: true,
    color: mix(PALETTE.text3, LANE.fix.text, smoothstep(0.35, 1, u)),
  });
  ctx.restore();
}

// The depends bracket: from lane 3's start, over the lane, down into
// ruff-format's chip, with `depends = "ruff"` on its run.

const DEP_LABEL = 'depends = "ruff"';
const DEP = {
  /** The bracket's run, level with the label's x-height. */
  y: 386,
  /** Its foot on lane 3, just inside ruff's bar, and the chip's top. */
  x0: STEP.ruff.x0 + 8,
  top: LANES.rows[2] - LANES.barH / 2 - 2,
  chipTop: LANES.rows[2] - LANES.barH / 2 - 2,
  label: { x: 700, y: 394, size: 32 },
  radius: 8,
} as const;
const DEP_LABEL_END = DEP.label.x + monoWidth(DEP_LABEL, DEP.label.size);
/**
 * The spark carries ruff's ✔ along the bracket, so it is the ✔'s green
 * (paper is the user's own work), with a green-tinted core.
 */
const DEP_SPARK = PALETTE.green;
const DEP_SPARK_CORE = mix(PALETTE.green, "#ffffff", 0.6);

/** The bracket as a polyline from lane 3's start to above the chip's middle, and its length. */
function depPath(chipX: number): { pts: { x: number; y: number }[]; lens: number[] } {
  const x1 = chipX + chipWidth("ruff-format") / 2;
  const r = DEP.radius;
  const pts: { x: number; y: number }[] = [{ x: DEP.x0, y: DEP.top }];
  const corner = (cx: number, cy: number, a0: number, a1: number) => {
    for (let i = 0; i <= 6; i++) {
      const a = lerp(a0, a1, i / 6);
      pts.push({ x: cx + r * Math.cos(a), y: cy + r * Math.sin(a) });
    }
  };
  corner(DEP.x0 + r, DEP.y + r, Math.PI, 1.5 * Math.PI);
  corner(x1 - r, DEP.y + r, 1.5 * Math.PI, 2 * Math.PI);
  pts.push({ x: x1, y: DEP.chipTop });
  const lens = [0];
  for (let i = 1; i < pts.length; i++) lens.push(lens[i - 1] + Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y));
  return { pts, lens };
}

/** The point `d` px along the path. */
function pathAt(p: { pts: { x: number; y: number }[]; lens: number[] }, d: number): { x: number; y: number } {
  const L = p.lens[p.lens.length - 1];
  const dd = clamp(d, 0, L);
  let i = 1;
  while (i < p.lens.length - 1 && p.lens[i] < dd) i++;
  const k = (dd - p.lens[i - 1]) / (p.lens[i] - p.lens[i - 1] || 1);
  return { x: lerp(p.pts[i - 1].x, p.pts[i].x, k), y: lerp(p.pts[i - 1].y, p.pts[i].y, k) };
}

function drawDepends(ctx: CanvasRenderingContext2D, b: number, lt: number): void {
  const draw = swiftOut(progress(BEATS.queue, BEATS.queue + 0.75, b));
  const fade = 1 - smoothstep(BEATS.depends + 0.125, BEATS.depends + 0.75, b);
  if (draw <= 0 || fade <= 0) return;
  const chip = ganttAt(Math.min(b, BEATS.depends)).chips.find((c) => c.step.step === "ruff-format");
  const chipX = chip?.x ?? STEP["ruff-format"].x0;
  const path = depPath(chipX);
  const L = path.lens[path.lens.length - 1];
  // Waiting it is dashed and marching toward ruff-format; ruff's ✔ lights it; on b4 it pulls taut.
  const lit = smoothstep(BEATS.ruffDone, BEATS.ruffDone + 0.25, b);
  const taut = smoothstep(BEATS.depends - 0.0625, BEATS.depends, b);
  const snap = flare(b, BEATS.depends, 0.12);
  // It pulls taut in the colour of ruff's ✔, which the spark carries to it.
  const color = mix(mix(PALETTE.text3, PALETTE.text2, lit), DEP_SPARK, Math.max(taut * 0.6, snap));
  ctx.save();
  ctx.globalAlpha *= fade;
  // The label sits on the run, which breaks around it: the run draws on
  // through the gap and gives way there, under the label's plate, as the
  // label comes up.
  const gapA = DEP.label.x - 10;
  const gapB = DEP_LABEL_END + 10;
  const labelUp = progress(BEATS.queue + 0.25, BEATS.queue + 0.625, b);
  const labelIn = labelUp * (1 - smoothstep(BEATS.depends + 0.25, BEATS.depends + 0.75, b));
  const inGap = 1 - smoothstep(BEATS.queue + 0.25, BEATS.queue + 0.4375, b);
  ctx.strokeStyle = color;
  ctx.lineWidth = 2 + snap;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  const gap = lerp(7, 0, taut);
  if (gap > 0.01) ctx.setLineDash([8, gap]);
  ctx.lineDashOffset = -lt * 40;
  // One unbroken path, so its dashes march straight through, stroked inside
  // the label's gap or outside it.
  const strokeRun = (inside: boolean, a: number): void => {
    if (a <= 0) return;
    ctx.save();
    ctx.beginPath();
    if (!inside) ctx.rect(-1e4, -1e4, 2e4, 2e4);
    ctx.rect(gapA, DEP.y - 8, gapB - gapA, 16);
    ctx.clip(inside ? "nonzero" : "evenodd");
    ctx.globalAlpha *= a;
    ctx.beginPath();
    const n = 120;
    for (let i = 0; i <= n; i++) {
      const p = pathAt(path, (L * draw * i) / n);
      if (i === 0) ctx.moveTo(p.x, p.y);
      else ctx.lineTo(p.x, p.y);
    }
    ctx.stroke();
    ctx.restore();
  };
  strokeRun(true, inGap);
  // The label's plate, of the stage's own colour, so the playhead passes behind it.
  if (labelIn > 0) {
    ctx.save();
    ctx.globalAlpha *= labelIn / fade;
    roundedRect(ctx, gapA + 4, DEP.y - 18, gapB - gapA - 8, 34, 6);
    ctx.fillStyle = PALETTE.bg;
    ctx.fill();
    ctx.restore();
  }
  strokeRun(false, 1);
  ctx.setLineDash([]);
  // The head, as the bracket reaches the chip.
  const headIn = progress(0.85, 1, draw);
  if (headIn > 0) {
    const tip = path.pts[path.pts.length - 1];
    ctx.fillStyle = rgba(color, headIn);
    ctx.beginPath();
    ctx.moveTo(tip.x, tip.y + 2);
    ctx.lineTo(tip.x - 7, tip.y - 9);
    ctx.lineTo(tip.x + 7, tip.y - 9);
    ctx.closePath();
    ctx.fill();
  }
  // The spark that carries ruff's ✔ along it to ruff-format, lighting the
  // bracket behind it.
  const u = progress(BEATS.dependsSpark, BEATS.depends, b);
  if (u > 0 && u < 1) {
    const d = L * inOutSine(u);
    ctx.save();
    ctx.setLineDash([]);
    ctx.lineCap = "round";
    for (let i = 0; i < 12; i++) {
      const d0 = d - 110 * (1 - i / 12);
      const d1 = d - 110 * (1 - (i + 1) / 12);
      if (d1 <= 0) continue;
      const p0 = pathAt(path, Math.max(0, d0));
      const p1 = pathAt(path, d1);
      if (Math.abs(p1.y - DEP.y) < 0.5 && p1.x > gapA && p1.x < gapB) continue;
      ctx.strokeStyle = rgba(DEP_SPARK, 0.9 * ((i + 1) / 12) ** 2);
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(p0.x, p0.y);
      ctx.lineTo(p1.x, p1.y);
      ctx.stroke();
    }
    const head = pathAt(path, d);
    glow(ctx, head.x, head.y, 34, DEP_SPARK, 0.8);
    ctx.fillStyle = DEP_SPARK_CORE;
    ctx.beginPath();
    ctx.arc(head.x, head.y, 4.5, 0, TAU);
    ctx.fill();
    ctx.restore();
  }
  ctx.globalAlpha *= labelIn / fade;
  // The spark reads the label through as it passes under it, glyph by glyph.
  const base = mix(PALETTE.text3, PALETTE.text2, lit);
  const sparkX = u > 0 && u < 1 ? pathAt(path, L * inOutSine(u)).x : -1e4;
  const size = DEP.label.size;
  Array.from(DEP_LABEL).forEach((ch, i) => {
    if (ch === " ") return;
    const x = DEP.label.x + i * 0.6 * size;
    const k = Math.exp(-(((x + 0.3 * size - sparkX) / 34) ** 2));
    drawMono(ctx, ch, x, DEP.label.y, size, k > 0.01 ? mix(base, DEP_SPARK, k) : base);
  });
  ctx.restore();
}

// deploy.sh changes hands: shfmt lets its lock go, and the padlock sends
// its key under lane 4 to shellcheck's leading edge, which holds it as the
// padlock shuts again. Under the lane, the key stays clear of shfmt's ✔
// badge on the bar's top corner, which pops on the same beat.

const KEY = { at: BEATS.shfmtDone, to: BEATS.shellcheckLock } as const;
/** From the padlock's body, where its keyhole would be… */
const KEY_FROM = { x: LANES.lockX + 2, y: LANES.rows[3] + 12 };
/** …to the middle of shellcheck's leading edge. */
const KEY_TO = { x: STEP.shellcheck.x0 + 6, y: LANES.rows[3] };
/** Its low point runs just under lane 4's track. */
const KEY_LOW = LANES.rows[3] + LANES.trackH / 2 + 16;
const KEY_HOP: Curve = { a: KEY_FROM, c: { x: (KEY_FROM.x + KEY_TO.x) / 2, y: 2 * KEY_LOW - (KEY_FROM.y + KEY_TO.y) / 2 }, b: KEY_TO };
/** The ring it lands with: small enough to clear the badge. */
const KEY_RING = 22;
/**
 * Steps on a lane that are handed their lock (shellcheck): the kit fades
 * their chip on their start, but it holds here until the key lands.
 */
const KEYED = SCHEDULE.filter((s) => !inDock(s) && lockedFrom(s) > s.start);

// Flashes down a column: the playhead dropping in, and the two slams.

function columnFlash(ctx: CanvasRenderingContext2D, x: number, b: number, at: number, color: string): void {
  if (b < at) return;
  const u = progress(at, at + 0.125, b);
  const a = u < 1 ? 1 : flare(b, at + 0.125, 0.12);
  if (a <= 0) return;
  const y0 = PLAYHEAD_LINE.y0;
  const head = lerp(y0, PLAYHEAD_LINE.y1, swiftOut(u));
  ctx.save();
  const g = ctx.createLinearGradient(0, y0, 0, head);
  g.addColorStop(0, rgba(color, 0));
  g.addColorStop(1, rgba(color, 0.85 * a));
  ctx.fillStyle = g;
  ctx.fillRect(x - 2, y0, 4, head - y0);
  if (u < 1) glow(ctx, x, head, 46, color, 0.9);
  edgeGlow(ctx, x, y0, head, color, 0.18 * a, 40);
  ctx.restore();
}

/**
 * A shock ring leaving a mark's edge: it is born at `r0`, already clear of
 * the mark (so the two never cross into a ⊘), eases out to `r1` and fades,
 * coming up over its first moment instead of popping on.
 */
function shockRing(ctx: CanvasRenderingContext2D, x: number, y: number, r0: number, r1: number, p: number, color: string, width = 4): void {
  if (p <= 0 || p >= 1) return;
  const e = 1 - (1 - p) ** 3;
  ctx.save();
  ctx.strokeStyle = rgba(color, (1 - p) ** 1.5 * smoothstep(0, 0.08, p));
  ctx.lineWidth = width * (1 - p) + 0.5;
  ctx.beginPath();
  ctx.arc(x, y, lerp(r0, r1, e), 0, TAU);
  ctx.stroke();
  ctx.restore();
}

/**
 * The cascade's rings: born outside the ✔ at its 41% overshoot (its tip is
 * 26 px out), and out to 34, inside the stage's right margin and clear of
 * newlines' right edge.
 */
const CASCADE_RING = { from: 27, to: 34 } as const;

/** The corner badge on a finished bar: a green ✔ on a dark disc over its top-right corner. */
function drawBadge(ctx: CanvasRenderingContext2D, x: number, y: number, pop: number, alpha: number): void {
  if (pop <= 0 || alpha <= 0) return;
  ctx.save();
  ctx.globalAlpha *= alpha;
  ctx.translate(x, y);
  ctx.scale(pop, pop);
  ctx.beginPath();
  ctx.arc(0, 0, 16, 0, TAU);
  ctx.fillStyle = PALETTE.bg;
  ctx.fill();
  ctx.strokeStyle = rgba(PALETTE.green, 0.9);
  ctx.lineWidth = 2;
  ctx.stroke();
  drawDone(ctx, 0, 1, { size: 24 });
  ctx.restore();
}

const DETAIL_TEXT = "Order from one real commit. Not to scale.";
const DETAIL_STYLE = wordStyle(40, PALETTE.text3);
const DETAIL_AT = { x: 640, y: 690 } as const;

/** Every bar the chart shows at `b`, where it has reached, and how it moves. */
function drawBars(ctx: CanvasRenderingContext2D, b: number, lt: number): void {
  const ph = playheadX(b);
  for (const s of SCHEDULE) {
    // A bar starts when its step holds its locks.
    const from = lockedFrom(s);
    if (b < from) continue;
    let x1 = clamp(ph, s.x0, s.x1);
    // A step handed its lock by another (a docked step's landing, shellcheck's
    // key) gets it a little after the playhead has passed its start: its bar
    // catches up over a 32nd instead of appearing part-grown.
    if (from > s.start) x1 = lerp(s.x0, x1, swiftOut(progress(from, from + 0.125, b)));
    // Home with a thunk: a few px past its end, back, and still.
    if (b >= s.end) x1 = s.x1 + 5 * jolt(b, s.end, 0.375, 2);
    // A docked step's name is on its reservation until its bar is whole (drawGhostLabel).
    if (inDock(s) && b < s.end) drawBar(ctx, barRect(s, x1), "fix");
    else drawScheduleBar(ctx, s, x1);
    const r = barRect(s, x1);
    const h = r.y1 - r.y0;
    const running = b < s.end;
    const edge = running ? 1 : 1 - progress(s.end, s.end + 0.25, b);
    if (edge > 0 && x1 - s.x0 > 1) {
      ctx.save();
      // A sheen travelling the bar toward its write head, once a beat.
      roundedRect(ctx, r.x0, r.y0, r.x1 - r.x0, h, LANES.barRadius);
      ctx.clip();
      const band = 110;
      const u = (((lt / BEAT + s.x0 / 400) % 1) + 1) % 1;
      const cx = r.x0 - band + u * (r.x1 - r.x0 + 2 * band);
      const g = ctx.createLinearGradient(cx - band, 0, cx + band, 0);
      g.addColorStop(0, rgba(PALETTE.warmBright, 0));
      g.addColorStop(0.5, rgba(PALETTE.warmBright, 0.1 * edge));
      g.addColorStop(1, rgba(PALETTE.warmBright, 0));
      ctx.fillStyle = g;
      ctx.fillRect(r.x0, r.y0, r.x1 - r.x0, h);
      // The write head at the tip.
      ctx.fillStyle = rgba(PALETTE.warmBright, 0.85 * edge);
      ctx.fillRect(r.x1 - 4, r.y0 + 6, 2.5, h - 12);
      ctx.restore();
      edgeGlow(ctx, r.x1 - 3, r.y0 + 10, r.y1 - 10, PALETTE.warm, 0.2 * edge);
    }
    // Done: the bar flashes.
    const f = flare(b, s.end, 0.15);
    if (f > 0) {
      ctx.save();
      roundedRect(ctx, r.x0, r.y0, r.x1 - r.x0, h, LANES.barRadius);
      ctx.fillStyle = rgba(PALETTE.warmBright, 0.4 * f);
      ctx.fill();
      ctx.restore();
    }
  }
}

/** Once the run is done, a light crosses the finished chart, over every bar. */
function drawGlint(ctx: CanvasRenderingContext2D, b: number): void {
  const u = progress(BEATS.glint[0], BEATS.glint[1], b);
  if (u <= 0 || u >= 1) return;
  const band = 150;
  const cx = lerp(LANES.trackX0 - band, LANES.trackX1 + band, inOutSine(u));
  const a = 0.16 * Math.sin(Math.PI * u);
  ctx.save();
  ctx.beginPath();
  for (const st of SCHEDULE) {
    const r = barRect(st, st.x1);
    const q = LANES.barRadius;
    ctx.moveTo(r.x0 + q, r.y0);
    ctx.arcTo(r.x1, r.y0, r.x1, r.y1, q);
    ctx.arcTo(r.x1, r.y1, r.x0, r.y1, q);
    ctx.arcTo(r.x0, r.y1, r.x0, r.y0, q);
    ctx.arcTo(r.x0, r.y0, r.x1, r.y0, q);
    ctx.closePath();
  }
  ctx.clip();
  const g = ctx.createLinearGradient(cx - band, 0, cx + band, 0);
  g.addColorStop(0, rgba(PALETTE.warmBright, 0));
  g.addColorStop(0.5, rgba(PALETTE.warmBright, a));
  g.addColorStop(1, rgba(PALETTE.warmBright, 0));
  ctx.fillStyle = g;
  ctx.fillRect(cx - band, 0, 2 * band, 1080);
  ctx.restore();
}

/** The badges, gathered into the cascade as it lands. */
function drawBadges(ctx: CanvasRenderingContext2D, b: number): void {
  const gather = 1 - smoothstep(CASCADE.beat + 0.25, CASCADE.beat + 1, b);
  for (const s of SCHEDULE) {
    if (s.end >= CASCADE.beat || b < s.end) continue;
    const { y0 } = barSpan(s.lanes[0], s.lanes[1]);
    drawBadge(ctx, s.x1 - 14, y0, land(b, s.end, 0.25, 0.25), gather);
  }
}

function draw(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  const b = lt / BEAT;
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, env.W, env.H);
  if (b >= SETTLED) {
    drawHandoff(ctx, "lanes|restore", env);
    return;
  }
  const g = ganttAt(b);
  // The slams jolt the chart down and back; the dock and the tray, nearer
  // and farther, move a little more and a little less.
  const kick = 4 * (jolt(b, BEATS.slam, 0.6, 2.5) + jolt(b, BEATS.slam2, 0.6, 2.5));

  // A broad, low warm bloom behind the chart on each slam.
  for (const at of [BEATS.slam, BEATS.slam2]) {
    const f = flare(b, at, 0.3);
    if (f <= 0) continue;
    // Twice as wide as it is tall, so it stays above the captions' band.
    ctx.save();
    ctx.translate(1200, 390);
    ctx.scale(2, 1);
    glow(ctx, 0, 0, 340, PALETTE.warm, 0.12 * f);
    ctx.restore();
  }

  ctx.save();
  if (kick !== 0) ctx.translate(0, kick);

  // Labels, tracks and padlocks, the padlocks punching as they snap.
  drawLanes(ctx, {
    locks: g.locks.map((l, lane) => {
      const scale = lockScale(lane, b, l.state === "write");
      return { state: l.state, lift: l.lift, scale: scale === 1 ? undefined : scale };
    }),
  });
  // The files a fixer has just written flash: warm at the clamp, green as the cascade passes them.
  LANE_FILES.forEach((file, lane) => {
    const cy = LANES.rows[lane] + 14;
    const warm = lane <= 1 ? flare(b, BEATS.clamp, 0.5) : 0;
    const green = flare(b, BEATS.cascade[lane], 0.3);
    if (warm > 0) {
      const w = monoWidth(file, 40);
      glowLine(ctx, LANES.labelX + 20, cy - 14, LANES.labelX + w - 20, cy - 14, PALETTE.warm, 0.22 * warm, 44);
      drawMono(ctx, file, LANES.labelX, cy, 40, rgba(PALETTE.warm, Math.min(1, 1.4 * warm)));
    }
    if (green > 0) drawMono(ctx, file, LANES.labelX, cy, 40, rgba(PALETTE.green, 0.8 * green));
  });

  for (const s of DOCKED) drawGhost(ctx, s, b, lt);
  drawBars(ctx, b, lt);
  for (const s of DOCKED) drawGhostLabel(ctx, s, b);
  drawGlint(ctx, b);
  // The playhead, over the bars' write heads: it drops in at go, a soft halo round the line.
  if (g.playhead.alpha > 0) {
    const a = g.playhead.alpha / PLAYHEAD_LINE.alpha;
    const y1 = lerp(PLAYHEAD_LINE.y0, PLAYHEAD_LINE.y1, swiftOut(progress(BEATS.go, BEATS.go + 0.125, b)));
    ctx.save();
    const halo = ctx.createLinearGradient(g.playhead.x - 14, 0, g.playhead.x + 14, 0);
    halo.addColorStop(0, rgba(PALETTE.cyan, 0));
    halo.addColorStop(0.5, rgba(PALETTE.cyan, 0.12 * a));
    halo.addColorStop(1, rgba(PALETTE.cyan, 0));
    ctx.fillStyle = halo;
    ctx.fillRect(g.playhead.x - 14, PLAYHEAD_LINE.y0, 28, y1 - PLAYHEAD_LINE.y0);
    ctx.fillStyle = rgba(PALETTE.cyan, g.playhead.alpha);
    ctx.fillRect(g.playhead.x - PLAYHEAD_LINE.width / 2, PLAYHEAD_LINE.y0, PLAYHEAD_LINE.width, y1 - PLAYHEAD_LINE.y0);
    ctx.restore();
  }

  // Waiting chips on the lanes: they pop on, march, and ride ahead of the
  // playhead. A chip waiting for a key stays solid until the key lands on
  // it, flushes warm, and gives way to its bar as the bar grows in under it.
  const pop = 0.85 + 0.15 * land(b, BEATS.queue, 0.3, 0.4);
  for (const c of g.chips) {
    if (c.dock || KEYED.includes(c.step)) continue;
    const w = chipWidth(c.step.step);
    ctx.save();
    ctx.translate(c.x + w / 2, c.cy);
    ctx.scale(pop, pop);
    drawWaitingChip(ctx, -w / 2, 0, c.step.step, { alpha: c.alpha, dashOffset: -lt * 36 });
    ctx.restore();
  }
  for (const s of KEYED) {
    const c = ganttAt(Math.min(b, s.start)).chips.find((k) => k.step === s);
    const to = lockedFrom(s);
    const alpha = (c?.alpha ?? 0) * (1 - progress(to, to + 0.125, b));
    if (!c || alpha <= 0) continue;
    const w = chipWidth(s.step);
    const flush = smoothstep(to - 0.0625, to, b);
    ctx.save();
    ctx.translate(c.x + w / 2, c.cy);
    ctx.scale(pop, pop);
    if (flush > 0) glow(ctx, -w / 2 + 4, 0, 40, PALETTE.warm, 0.5 * flush * alpha);
    drawWaitingChip(ctx, -w / 2, 0, s.step, { alpha, dashOffset: -lt * 36, stroke: mix(PALETTE.text3, PALETTE.warmBright, flush) });
    ctx.restore();
  }
  drawDepends(ctx, b, lt);

  // deploy.sh's key, from its padlock to shellcheck.
  const ku = progress(KEY.at, KEY.to, b);
  if (ku > 0 && ku < 1) drawSpark(ctx, KEY_HOP, inOutSine(ku), { color: PALETTE.warmBright, size: 7, trail: 0.3 });
  // Its ring is gone before shellcheck's label comes up under it.
  if (b >= KEY.to) ring(ctx, KEY_TO.x, KEY_TO.y, KEY_RING, progress(KEY.to, KEY.to + 0.3, b), PALETTE.warm, 4);

  drawBadges(ctx, b);
  // The cascade: a ✔ per lane at x 1730, each with a ring.
  g.checks.forEach((k, lane) => {
    if (k <= 0) return;
    const cy = LANES.rows[lane];
    const at = BEATS.cascade[lane];
    glow(ctx, CASCADE.x, cy, 60, PALETTE.green, 0.4 * flare(b, at, 0.25));
    shockRing(ctx, CASCADE.x, cy, CASCADE_RING.from, CASCADE_RING.to, progress(at, at + 0.6, b), PALETTE.green);
    drawDone(ctx, CASCADE.x, cy, { size: CASCADE.size, scale: k });
  });

  columnFlash(ctx, STEP.prettier.x0, b, BEATS.go, PALETTE.cyanBright);
  columnFlash(ctx, STEP["trailing-whitespace"].x0, b, BEATS.slam, PALETTE.warmBright);
  columnFlash(ctx, STEP.newlines.x0, b, BEATS.slam2, PALETTE.warmBright);

  // A warm flare on each padlock as it shuts.
  LANES.rows.forEach((cy, lane) => {
    const e = lastLockEvent(lane, b);
    const knock = knockOn(b, g.locks[lane].state === "write").flare;
    const shut = e?.shut ? flare(b, e.at, 0.2) : 0;
    glow(ctx, LANES.lockX, cy, 56, PALETTE.warm, Math.max(0.6 * shut, 0.45 * knock));
  });
  ctx.restore();

  // The dock, nearer: it jolts a little more.
  ctx.save();
  if (kick !== 0) ctx.translate(0, kick * 1.5);
  for (const s of DOCKED) if (b < s.start) drawDockChip(ctx, s, b, lt);
  ctx.restore();

  // The detail line: it fades up under the first caption and wipes at b14.5.
  const da = smoothstep(1.5, 2.5, b);
  if (da > 0 && b < BEATS.detailOut + 0.25) {
    ctx.save();
    ctx.globalAlpha *= da;
    drawWords(ctx, DETAIL_TEXT, DETAIL_AT.x, DETAIL_AT.y + 8 * (1 - swiftOut(progress(1.5, 2.5, b))), DETAIL_STYLE, lt, 0, BEATS.detailOut * BEAT);
    ctx.restore();
  }

  // The tray, farther back: it jolts a little less.
  ctx.save();
  if (kick !== 0) ctx.translate(0, kick * 0.5);
  drawTray(ctx, { holding: true, t: env.t });
  ctx.restore();
}

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw,
  lit: () => null,
  captions: () => CAPTIONS,
};
