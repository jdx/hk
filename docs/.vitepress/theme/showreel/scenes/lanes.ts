// Section 5, "Fixers in parallel" (storyboard §6.5): the heart of the reel.
// hk runs every step whose files are free at once, and a fixer that works
// out its fix as a diff (`check_diff`) only reads its files while it does:
// any number of steps can read a file together, and only writing one needs
// it alone. The pre-commit run of the commit capture plays on the four file
// lanes from kit/lanes.ts SCHEDULE, read phases cyan and write phases warm:
// prettier and ruff fix their own files side by side while shfmt and
// shellcheck read deploy.sh together; shfmt, holding its patch, waits for
// shellcheck to finish reading before hk writes it; ruff-format follows
// ruff, which it depends on, reads main.py and takes the write lock only to
// apply its patch; and trailing-whitespace and newlines, which read every
// file, wait in the dock for prettier's files and then read all four
// together.
//
// What the viewer reads, beat by beat (lanes-local beats, BEATS in lanes-timing.ts):
//
//   b0.5   the queue: the two all-file steps slide into the dock, the
//          waiting ruff-format chip pops onto lane 3, and a dashed bracket
//          ties it to lane 3's start: depends = "ruff"
//   b1     all hands: the playhead drops in at x 648, the chart takes the
//          hit, all four padlocks snap shut together, deploy.sh's cyan
//          with two pips, and four bars surge out of it at once: prettier
//          and ruff warm, shfmt and shellcheck cyan, a half of lane 4 each
//   b2     shfmt has its patch: its read bar ends, a pip goes out, and a
//          dashed tie runs on while it waits holding nothing
//   b3     shellcheck ✔, with nothing to write: deploy.sh's padlock opens
//          for a sixteenth and sends a warm key over lane 4 to the start
//          of shfmt's write, and shuts warm as it lands on b3.25
//   b4     shfmt ✔ and deploy.sh flashes warm: written. b4.5 ruff ✔
//          writes main.py and sends a green spark along the bracket,
//          reading its label through; b5 it snaps taut and fades, and
//          ruff-format starts reading
//   b7.5   ruff-format has its patch and trades its read lock for main.py's
//          write lock; b8.5 ruff-format ✔ writes main.py
//   b10    prettier ✔ writes README.md and src/app.ts and lets them go,
//          and trailing-whitespace and newlines drop out of the dock
//          together across all four lanes, landing on b10.25 as every
//          padlock shuts cyan with two pips: a dashed outline of the whole
//          bar, both names up the middle, which the bar then grows into
//   b12    both ✔, having found nothing to fix; a green ✔ cascade down
//          x 1730, padlocks open
//   b14.5  the detail wipes; from b15 the frame is the lanes|restore
//          handoff (the finished Gantt and the closed tray)
//
// Running bars glow at the playhead and carry a sheen toward it, in their
// phase's colour; a finished bar flashes, thunks a few px past its end and
// settles, and wears a ✔ badge on its corner until the cascade gathers them
// at b12. A file flashes warm when a fixer's ✔ row said it modified it
// (kit/lanes.ts WROTE). Waiting chips march, and the docked ones breathe.
// A light crosses the finished chart in the hold. Every frame is a pure
// function of the beat: the chart's state comes from ganttAt(), and
// everything drawn over it is timed from SCHEDULE and BEATS.

import { BEAT, LANE, PALETTE, type Scene, type SceneEnv, sec } from "../bible";
import { mix, rgba } from "../color";
import { glow, ring, roundedRect } from "../fx";
import { drawHandoff } from "../handoff";
import { drawMono, monoWidth } from "../kit/card";
import {
  badgeY,
  CASCADE,
  chipWidth,
  DOCK,
  DOCK_CHIP,
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
  type LockChange,
  lockChanges,
  lockedFrom,
  phaseKind,
  phaseLockedFrom,
  PLAYHEAD_LINE,
  SCHEDULE,
  type ScheduleStep,
  stepReach,
  stepSpan,
  twinDx,
  twinIndex,
  WROTE,
} from "../kit/lanes";
import { type Curve, drawSpark, jolt, land } from "../kit/motion";
import { drawTray } from "../kit/tray";
import { clamp, inOutSine, inQuad, lerp, progress, smoothstep, swiftOut, TAU } from "../math";
import { type Caption, drawWords, font, layout, wordStyle } from "../type";
import { BEATS } from "./lanes-timing";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 4 words: need 3, hold 4, landing a beat after the four steps start.
  { out: 6, lines: [{ in: 2, text: "Fixers run in parallel." }] },
  // 6 words: need 4, hold 4.5, up as ruff-format trades its read lock for a
  // write lock, and while the two steps that read every file wait for
  // prettier's files, then share all four.
  { out: 12.5, lines: [{ in: 8, text: "File locks keep them from colliding." }] },
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

/** A step's rect as drawScheduleBar draws it, from its start to `x1`: its lanes' band, or the half a shared reader takes. */
function barRect(s: ScheduleStep, x1: number): { x0: number; x1: number; y0: number; y1: number } {
  const { y0, y1 } = stepSpan(s);
  return { x0: s.x0, x1, y0, y1 };
}

/** Phase `i` of step `s`, to `x1` at most. */
function phaseRect(s: ScheduleStep, i: number, x1: number): { x0: number; x1: number; y0: number; y1: number } {
  const { y0, y1 } = stepSpan(s);
  return { x0: s.phases[i].x0, x1, y0, y1 };
}

/** The colour of a phase's light: cyan for a read, warm for a write. */
const phaseColor = (s: ScheduleStep, i: number): { base: string; bright: string } =>
  s.phases[i].lock === "read" ? { base: PALETTE.cyan, bright: PALETTE.cyanBright } : { base: PALETTE.warm, bright: PALETTE.warmBright };

// The padlocks: each shut and each opening gets a punch, a reader joining
// or leaving a small one, and a shut a flare, warm for a writer and cyan for
// readers. The events are the kit's own lock changes.

const LOCK_EVENTS: readonly LockChange[][] = LANES.rows.map((_, lane) => lockChanges(lane));
const lastLockEvent = (lane: number, b: number): LockChange | null =>
  LOCK_EVENTS[lane].reduce<LockChange | null>((m, e) => (e.at <= b ? e : m), null);

/**
 * The padlock's scale: a punch that overshoots and settles, exactly 1 half
 * a beat on. The four that shut together at go punch hardest.
 */
function lockScale(lane: number, b: number): number {
  const e = lastLockEvent(lane, b);
  if (!e) return 1;
  const punch = e.change === "open" ? 0.16 : e.change === "pips" ? 0.1 : e.at === BEATS.go ? 0.5 : 0.34;
  return 1 + punch * jolt(b, e.at, 0.5, 3);
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

/** How a docked chip sits at `b`: its slide in, idle bob and the small wind-up before it drops. */
function dockPose(s: ScheduleStep, b: number): { dx: number; dy: number; sx: number; sy: number; alpha: number } {
  const k = dockOrder(s);
  const inAt = BEATS.queue + k / 8;
  const alpha = progress(inAt, inAt + 0.25, b);
  const dx = 96 * (1 - swiftOut(progress(inAt, inAt + 0.625, b)));
  // Waiting: a slow bob, the queue breathing together.
  let dy = 2 * Math.sin(TAU * (b / 4)) * smoothstep(1, 2, b);
  // The wind-up: it rises and squashes a little over the sixteenth before it drops.
  const wind = smoothstep(s.start - 0.25, s.start, b);
  dy -= 4 * wind;
  return { dx, dy, sx: 1 + 0.02 * wind, sy: 1 - 0.06 * wind, alpha };
}

/** A docked chip's outline: any waiting chip's. */
const DOCK_STROKE = LANE.waiting.stroke;

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
  ctx.strokeStyle = DOCK_STROKE;
  ctx.lineWidth = 2;
  ctx.stroke();
  ctx.restore();
  ctx.save();
  ctx.translate(x + w / 2 + p.dx, DOCK_CY + p.dy);
  ctx.scale(p.sx, p.sy);
  drawWaitingChip(ctx, -w / 2, 0, s.step, { ...DOCK_CHIP, alpha: p.alpha, outline: false });
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

/**
 * The docked chip's rect and pose as it leaves the dock, and the rect it
 * falls into. Twins leave together, as one blind round both their chips.
 */
function diveRects(s: ScheduleStep): { from: { x0: number; x1: number; y0: number; y1: number }; full: { x0: number; x1: number; y0: number; y1: number } } {
  const own = dockRect(s, s.start);
  if (!s.twin) return { from: own, full: barRect(s, s.x1) };
  const pair = dockRect(STEP[s.twin], s.start);
  const from = { x0: Math.min(own.x0, pair.x0), x1: Math.max(own.x1, pair.x1), y0: Math.min(own.y0, pair.y0), y1: Math.max(own.y1, pair.y1) };
  return { from, full: barRect(s, s.x1) };
}

/**
 * Where twins' blind is split between their chips at dive progress `u`:
 * from the middle of the gap between them in the dock to the middle of
 * their bar, moving with the blind's sides. Each name holds its own side.
 */
function twinSeam(s: ScheduleStep, u: number): number {
  const [a, b] = [dockRect(s, s.start), dockRect(STEP[s.twin!], s.start)].sort((p, q) => p.x0 - q.x0);
  const { full } = diveRects(s);
  return lerp((a.x1 + b.x0) / 2, (full.x0 + full.x1) / 2, swiftOut(u));
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
  const landed = flare(b, to, 0.2);
  const fade = 1 - progress(s.end, s.end + 0.25, b);
  // It lands as the phase it reserves: cyan for readers.
  const light = phaseColor(s, 0);
  // Twins share one blind: the first draws it, and each draws its own chip's name.
  const blind = twinIndex(s) !== 1;
  ctx.save();
  ctx.globalAlpha *= fade;
  // Falling light at the blind's foot.
  if (blind && u < 1) glow(ctx, (r.x0 + r.x1) / 2, r.y1, 44, light.base, 0.35 * smoothstep(0.3, 0.9, u));
  roundedRect(ctx, r.x0, r.y0, w, r.y1 - r.y0, LANES.barRadius);
  if (blind) {
    // The chip has no fill; the reservation takes a faint one as it falls.
    ctx.fillStyle = rgba(light.base, 0.05 * smoothstep(0, 0.6, u) + 0.05 * landed);
    ctx.fill();
    ctx.setLineDash([...LANE.waiting.dash]);
    ctx.lineDashOffset = -lt * 36;
    ctx.strokeStyle = mix(DOCK_STROKE, light.bright, Math.max(0.5 * smoothstep(0, 1, u) * (1 - progress(to, to + 1, b)), 0.6 * landed));
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.setLineDash([]);
  }
  // The chip's name and its small padlock ride the blind's top right corner
  // in the chip's wound-up squash, so the first frame of the dive is the
  // last frame in the dock; a twin's ride its side of the pair's blind, up
  // to the seam between them. The lock it waited for is free: the shackle
  // springs open, and both fade as the blind drops.
  const ca = GHOST_CHIP_OUT(u);
  if (ca > 0) {
    ctx.save();
    ctx.clip();
    const own = dockRect(s, at);
    const cw = chipWidth(s.step, DOCK_CHIP);
    const sx = (own.x1 - own.x0) / cw;
    const sy = (own.y1 - own.y0) / DOCK_CHIP.h;
    const left = s.twin !== undefined && own.x0 === from.x0;
    if (s.twin) {
      const seam = twinSeam(s, u);
      ctx.beginPath();
      if (left) ctx.rect(r.x0 - 1, r.y0 - 1, seam - r.x0 + 1, r.y1 - r.y0 + 2);
      else ctx.rect(seam, r.y0 - 1, r.x1 - seam + 1, r.y1 - r.y0 + 2);
      ctx.clip();
    }
    ctx.translate(left ? r.x0 + (cw * sx) / 2 : r.x1 - (cw * sx) / 2, r.y0 + (DOCK_CHIP.h * sy) / 2);
    ctx.scale(sx, sy);
    drawWaitingChip(ctx, -cw / 2, 0, s.step, {
      ...DOCK_CHIP,
      alpha: ca,
      outline: false,
      lockLift: land(b, at, 0.0625, 0.3),
    });
    ctx.restore();
  }
  ctx.restore();
  // It lands softly along its foot.
  if (blind && landed > 0) glowLine(ctx, full.x0, full.y1, full.x1, full.y1, light.base, 0.18 * landed * fade, 30);
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
  const kind = phaseKind(s.phases[0]);
  drawBarLabel(ctx, { x0: r.x0, x1: r.x1, y0: full.y0 + drop, y1: full.y1 + drop }, kind, s.step, {
    rotate: true,
    dx: twinDx(s),
    color: mix(PALETTE.text3, (kind === "check" ? LANE.check : LANE.fix).text, smoothstep(0.35, 1, u)),
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
  // Waiting it is dashed and marching toward ruff-format; ruff's ✔ lights it; on b5 (BEATS.depends) it pulls taut.
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

// deploy.sh changes hands: shellcheck finishes reading and lets its lock go,
// and the padlock sends its key over lane 4 to shfmt's write stub, which
// holds it as the padlock shuts again, warm: hk applies shfmt's patch.
// Over the lane, in the gap above its track, the key stays clear of
// shellcheck's ✔ badge under its bar, which pops on the same beat, and it
// has risen about 20 px clear of shfmt's read bar by the bar's start.

const SHFMT_WRITE = STEP.shfmt.phases[1];
const KEY = { at: BEATS.shellcheckDone, to: BEATS.shfmtWrite } as const;
/** From the padlock's body, where its keyhole would be… */
const KEY_FROM = { x: LANES.lockX + 2, y: LANES.rows[3] + 12 };
/** …to the middle of shfmt's write stub's leading edge. */
const KEY_TO = { x: SHFMT_WRITE.x0 + 6, y: (stepSpan(STEP.shfmt).y0 + stepSpan(STEP.shfmt).y1) / 2 };
/**
 * Its control point stands 12 px into shfmt's read bar, 10 px above lane
 * 3's centre, so the hop rises steeply out of the padlock and clears the
 * bar's corner; its high point, about y 494, runs in the gap between lanes
 * 3 and 4, some 18 px under lane 3's track.
 */
const KEY_HOP: Curve = { a: KEY_FROM, c: { x: STEP.shfmt.phases[0].x0 + 12, y: LANES.rows[2] - 10 }, b: KEY_TO };
/** The ring it lands with: small enough to sit on the stub's half of the lane. */
const KEY_RING = 16;

// Flashes down a column: the playhead dropping in, and the readers' slam.

function columnFlash(ctx: CanvasRenderingContext2D, x: number, b: number, at: number, color: string, strength = 1): void {
  if (b < at) return;
  const u = progress(at, at + 0.125, b);
  const a = strength * (u < 1 ? 1 : flare(b, at + 0.125, 0.12));
  if (a <= 0) return;
  const y0 = PLAYHEAD_LINE.y0;
  const head = lerp(y0, PLAYHEAD_LINE.y1, swiftOut(u));
  ctx.save();
  const g = ctx.createLinearGradient(0, y0, 0, head);
  g.addColorStop(0, rgba(color, 0));
  g.addColorStop(1, rgba(color, 0.85 * a));
  ctx.fillStyle = g;
  ctx.fillRect(x - 2, y0, 4, head - y0);
  if (u < 1) glow(ctx, x, head, 46, color, 0.9 * strength);
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

/** The phase step `s` holds, or last held, at `b`: the last whose lock has shut. */
function phaseAt(s: ScheduleStep, b: number): number {
  let i = 0;
  for (let k = 0; k < s.phases.length; k++) if (b >= phaseLockedFrom(s, k)) i = k;
  return i;
}

/** Every bar the chart shows at `b`, where it has reached, and how it moves. */
function drawBars(ctx: CanvasRenderingContext2D, b: number, lt: number): void {
  for (const s of SCHEDULE) {
    // A bar starts when its step holds its locks, and each phase grows while
    // it holds them (stepReach), a phase handed its lock catching up.
    if (b < lockedFrom(s)) continue;
    let x1 = stepReach(s, b);
    // Home with a thunk: a few px past its end, back, and still.
    if (b >= s.end) x1 = s.x1 + 5 * jolt(b, s.end, 0.375, 2);
    // A docked step's name is on its reservation until its bar is whole (drawGhostLabel).
    drawScheduleBar(ctx, s, x1, 1, { labels: !(inDock(s) && b < s.end) });
    // A twin's light is its pair's: the first draws it.
    if (twinIndex(s) === 1) continue;
    const i = phaseAt(s, b);
    const p = s.phases[i];
    const color = phaseColor(s, i);
    const last = i === s.phases.length - 1;
    const r = phaseRect(s, i, last ? x1 : Math.min(x1, p.x1));
    const h = r.y1 - r.y0;
    const running = b < p.end;
    const edge = running ? 1 : last ? 1 - progress(s.end, s.end + 0.25, b) : 0;
    if (edge > 0 && r.x1 - r.x0 > 1) {
      ctx.save();
      // A sheen travelling the bar toward its head, once a beat.
      roundedRect(ctx, r.x0, r.y0, r.x1 - r.x0, h, LANES.barRadius);
      ctx.clip();
      const band = 110;
      const u = (((lt / BEAT + s.x0 / 400) % 1) + 1) % 1;
      const cx = r.x0 - band + u * (r.x1 - r.x0 + 2 * band);
      const g = ctx.createLinearGradient(cx - band, 0, cx + band, 0);
      g.addColorStop(0, rgba(color.bright, 0));
      g.addColorStop(0.5, rgba(color.bright, 0.1 * edge));
      g.addColorStop(1, rgba(color.bright, 0));
      ctx.fillStyle = g;
      ctx.fillRect(r.x0, r.y0, r.x1 - r.x0, h);
      // The head at the tip: a read head or a write head.
      ctx.fillStyle = rgba(color.bright, 0.85 * edge);
      ctx.fillRect(r.x1 - 4, r.y0 + Math.min(6, h / 4), 2.5, h - 2 * Math.min(6, h / 4));
      ctx.restore();
      edgeGlow(ctx, r.x1 - 3, r.y0 + Math.min(10, h / 3), r.y1 - Math.min(10, h / 3), color.base, 0.2 * edge);
    }
    // A phase ending flashes: brightly as the step finishes, softly as a
    // reader with a patch lets its read lock go.
    s.phases.forEach((q, k) => {
      const done = k === s.phases.length - 1;
      const f = flare(b, q.end, 0.15) * (done ? 1 : 0.5);
      if (f <= 0) return;
      const rr = phaseRect(s, k, done ? x1 : q.x1);
      ctx.save();
      roundedRect(ctx, rr.x0, rr.y0, rr.x1 - rr.x0, rr.y1 - rr.y0, LANES.barRadius);
      ctx.fillStyle = rgba(phaseColor(s, k).bright, 0.4 * f);
      ctx.fill();
      ctx.restore();
    });
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
    st.phases.forEach((ph, i) => {
      const r = phaseRect(st, i, ph.x1);
      const q = Math.min(LANES.barRadius, (r.y1 - r.y0) / 2);
      ctx.moveTo(r.x0 + q, r.y0);
      ctx.arcTo(r.x1, r.y0, r.x1, r.y1, q);
      ctx.arcTo(r.x1, r.y1, r.x0, r.y1, q);
      ctx.arcTo(r.x0, r.y1, r.x0, r.y0, q);
      ctx.arcTo(r.x0, r.y0, r.x1, r.y0, q);
      ctx.closePath();
    });
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
    drawBadge(ctx, s.x1 - 14, badgeY(s), land(b, s.end, 0.25, 0.25), gather);
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
  // All hands: go jolts the chart down and back as four steps start at
  // once; the dock and the tray, nearer and farther, move a little more and
  // a little less.
  const kick = 5 * jolt(b, BEATS.go, 0.6, 2.5);

  // A broad, low warm bloom behind the steps as they start together.
  const surge = flare(b, BEATS.go, 0.3);
  if (surge > 0) {
    // Wider than it is tall, so it stays above the captions' band.
    ctx.save();
    ctx.translate(820, 380);
    ctx.scale(1.6, 1);
    glow(ctx, 0, 0, 300, PALETTE.warm, 0.16 * surge);
    ctx.restore();
  }

  ctx.save();
  if (kick !== 0) ctx.translate(0, kick);

  // Labels, tracks and padlocks, the padlocks punching as they snap.
  drawLanes(ctx, {
    locks: g.locks.map((l, lane) => {
      const scale = lockScale(lane, b);
      return { state: l.state, lift: l.lift, pips: l.pips, scale: scale === 1 ? undefined : scale };
    }),
  });
  // A file flashes warm as a fixer that wrote it finishes (WROTE: the ✔ rows'
  // `N files modified`), and green as the cascade passes it.
  LANE_FILES.forEach((file, lane) => {
    const cy = LANES.rows[lane] + 14;
    const warm = Math.max(0, ...SCHEDULE.filter((s) => WROTE[s.step].includes(lane)).map((s) => flare(b, s.end, 0.5)));
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
  // playhead.
  const pop = 0.85 + 0.15 * land(b, BEATS.queue, 0.3, 0.4);
  for (const c of g.chips) {
    if (c.dock) continue;
    const w = chipWidth(c.step.step);
    ctx.save();
    ctx.translate(c.x + w / 2, c.cy);
    ctx.scale(pop, pop);
    drawWaitingChip(ctx, -w / 2, 0, c.step.step, { alpha: c.alpha, dashOffset: -lt * 36 });
    ctx.restore();
  }
  drawDepends(ctx, b, lt);

  // deploy.sh's key, from its padlock to shfmt's write.
  const ku = progress(KEY.at, KEY.to, b);
  if (ku > 0 && ku < 1) drawSpark(ctx, KEY_HOP, inOutSine(ku), { color: PALETTE.warmBright, size: 7, trail: 0.3 });
  // It lands with a ring on the write's leading edge.
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
  // The steps that start together flare where their bars leave the line, each in its phase's colour.
  for (const s of SCHEDULE) {
    if (s.start !== BEATS.go) continue;
    const { y0, y1 } = stepSpan(s);
    const inset = Math.min(10, (y1 - y0) / 3);
    edgeGlow(ctx, s.x0 + 4, y0 + inset, y1 - inset, phaseColor(s, 0).base, 0.5 * surge, 40);
  }
  // The steps that read every file land together: one flash down their column.
  columnFlash(ctx, STEP["trailing-whitespace"].x0, b, BEATS.slam, PALETTE.cyanBright, 0.45);

  // A flare on each padlock as it shuts, warm for a writer and cyan for
  // readers, brightest as all four shut together at go.
  LANES.rows.forEach((cy, lane) => {
    const e = lastLockEvent(lane, b);
    const shut = e?.change === "shut" ? flare(b, e.at, 0.2) * (e.at === BEATS.go ? 0.9 : 0.6) : 0;
    glow(ctx, LANES.lockX, cy, 56, e?.state === "read" ? PALETTE.cyan : PALETTE.warm, shut);
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
