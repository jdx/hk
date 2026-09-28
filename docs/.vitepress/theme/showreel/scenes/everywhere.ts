// Section 8, "Commit, terminal, CI" (storyboard §6.8): two ideas in twelve
// beats.
//
// One set of steps. Three panels slam down out of the hk.pkl chip, one per
// beat, each hanging from it on a cyan connector: the pre-commit hook
// (`git commit`), a terminal (`hk fix`) and CI (`hk check --all`). Each
// lists the top of the final screen of a real run (kit/screens.ts `final`),
// its rows ticking on in a 32nd cascade, and all three name the same seven
// steps (the CI one shows a terminal's screen, stylised: see PANELS).
// Pulses run down the connectors on the eighths, from the one config into
// all three.
//
// Check shows the diff; fix applies it. The panels roll up, their
// connectors retract into the chip, which goes, and two small timelines for
// one file and one step draw in: ruff-format on src/main.py, whose builtin
// fixes through `check_diff` (`ruff format --diff`). Under `hk check` the
// step reads the file under a read lock and fails, a red ✗, and its diff
// unfolds under it: rows of ruff's own diff of the staged file
// (RUFF_FORMAT_DIFF). Under `hk fix` the same step runs the same command
// under a read lock too, a cyan pill the length of the check's, and its
// own diff comes up under it: ruff, which it depends on, has already
// removed the unused import there, so it is the diff of the file without
// it (RUFF_FORMAT_FIX_DIFF). Then the step trades the read lock for the
// write lock, a warm pill, and hk applies the diff itself: the removals are
// struck and fold away, the additions settle into the file as the commit
// got it, and the step ✔. Both modes run the same command: hk check
// prints its diff, hk fix applies its own, and only applying it needs the
// file to itself (src/step_job.rs relock_for_write, then src/step/diff.rs
// apply_patch). The beats live in everywhere-timing.ts, which the score
// can import.
//
// The section leaves on the whip into the race (whip.ts): from b10.5 the
// columns wind up, whip out to the left and the streaks take over, still
// moving on its last frame. Nothing here depends on the benchmark facts.

import { BEAT, type LitRect, PALETTE, type Scene, type SceneEnv, sec, TERM } from "../bible";
import { mix, rgba } from "../color";
import { glow, roundedRect } from "../fx";
import { drawChip, drawPanel as drawCardPanel, drawMono, HKPKL_CHIP, monoWidth, RUFF_FORMAT_DIFF, RUFF_FORMAT_FIX_DIFF } from "../kit/card";
import { BAR_FILL, type BarKind, drawBar, drawDone, drawPadlock, HANDOFF_GAP, LANES, type LockMode } from "../kit/lanes";
import { bump, jolt, land, lerpRect, type Pt, type Rect, typedChars } from "../kit/motion";
import { final, type Screen } from "../kit/screens";
import { advance, drawTermLine, drawWindow, mini, termLit } from "../kit/term";
import { clamp, DEG, hash, inQuad, keys, lerp, outCubic, outQuad, progress, smoothstep, swiftOut, TAU } from "../math";
import { type Caption, drawWords, LABEL, wordStyle } from "../type";
import { drawWhip, drawWhipOut, WHIP_START } from "../whip";
import { drawCross } from "./catch-rig";
import {
  ALL_DONE,
  APPLY,
  CHECK_DONE,
  CHECK_GO,
  CHIP_OUT,
  COLUMN_DRAW,
  COLUMN_IN,
  DIFF_IN,
  DIFF_ROW,
  FIX_DIFF_ROW,
  FIX_DONE,
  FIX_GO,
  FIX_PATCH,
  FIX_WRITE,
  FOLD,
  FOLD_LEN,
  IMPACT,
  LAUNCH,
  PULSE_FLY,
  PULSES,
  rowLands,
  rowStart,
} from "./everywhere-timing";

const S = sec("everywhere");

/** Section-local seconds at beat `n`. */
const b = (n: number): number => n * BEAT;

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 4 + 3 words: need 4.5, hold 4.5 from b1 (line 2 needs 2.5, holds 3.5).
  // Each line lands on a slam, and it wipes away over b5.5–5.75 as the
  // panels start to roll up, clear before the next caption's first word.
  { out: 5.5, lines: [{ in: 1, text: "One set of steps:" }, { in: 2, text: "commit, terminal, CI." }] },
  // 4 + 3 words: need 4.5, hold 4.5; its last word lands on b6.5, as the
  // check starts, and its first rises on b6, after the caption above has
  // gone. Line 2 lands as hk fix has its patch and holds 3 beats, 2.5
  // needed.
  { out: 11, lines: [{ in: CHECK_GO, text: "Check shows the diff." }, { in: FIX_PATCH, text: "Fix applies it." }] },
];

// The beats, section-local (everywhere-timing.ts, which the score can
// import too).

/** A roll-up: a small tug down first, then up into the top edge, accelerating. */
const ROLL_UP = keys([
  [0, 0],
  [0.3, -0.03, outQuad],
  [1, 1, inQuad],
]);

// Layout, logical px.

interface PanelSpec {
  x: number;
  label: string;
  cmd: string;
  screen: Screen;
}

/**
 * The three panels (storyboard §6.8): a label, the command, and the top 9
 * rows of its run's last screen. The CI panel is stylised: it shows the
 * `hk check --all` terminal capture, header and progress bar included. In
 * CI hk prints plain text instead, with no header or bar and the ✔ lines
 * in the order steps finish, among `❯` and log lines (src/cli/mod.rs:
 * `is_ci` selects ProgressOutput::Text). The panel keeps the terminal's
 * screen so that all three show the same rows the same way; a verbatim CI
 * log would need a non-TTY capture in test/captures and kit/screens.ts.
 */
const PANELS: readonly PanelSpec[] = [
  { x: 160, label: "commit", cmd: "git commit", screen: final.commit },
  { x: 700, label: "terminal", cmd: "hk fix", screen: final.fix },
  { x: 1240, label: "CI", cmd: "hk check --all", screen: final.checkAll },
];
const PANEL_Y = 250;
const PANEL_W = 520;
const PANEL_H = 450;
const FLOOR_Y = PANEL_Y + PANEL_H;
/** Where the panel's terminal starts: under its label and command. */
const BODY_Y = 384;
const LABEL_Y = 316;
const CMD_Y = 364;
const CMD_SIZE = 40;
const panelRect = (i: number): Rect => ({ x: PANELS[i].x, y: PANEL_Y, w: PANEL_W, h: PANEL_H });
/** All three panels: the lit screen the vignette spares while they are up. */
const PANELS_LIT: Rect = { x: PANELS[0].x, y: PANEL_Y, w: PANELS[2].x + PANEL_W - PANELS[0].x, h: PANEL_H };

/** The connectors hang from the chip's bottom centre. */
const HUB: Pt = { x: HKPKL_CHIP.x + HKPKL_CHIP.w / 2, y: HKPKL_CHIP.y + HKPKL_CHIP.h };
/** A panel leaves the chip as a sliver under it. */
const SLIVER: Rect = { x: HUB.x - 90, y: HUB.y + 2, w: 180, h: 6 };

/** A stretch of the column's step holding the file: its lock, from when to when, and its pill's extent. */
interface Hold {
  lock: LockMode;
  from: number;
  to: number;
  x0: number;
  x1: number;
}

interface Column {
  x: number;
  title: string;
  color: string;
  lock: Pt;
  labelRight: number;
  x0: number;
  x1: number;
  at: number;
  /**
   * Its step's holds on the file, in order, each with its pill: a read
   * while the diff command runs, and under hk fix a write while hk applies
   * the diff. The reads' pills are the same length in both columns: the
   * same command.
   */
  holds: readonly Hold[];
}

/** A read's pill: half a track, x0 to x0 + 180. */
const READ_W = 180;
/** The two timelines for one file and one step (storyboard §6.8). */
const CHECK: Column = {
  x: 160,
  title: "hk check",
  color: PALETTE.cyan,
  lock: { x: 470, y: 308 },
  labelRight: 504,
  x0: 540,
  x1: 900,
  at: COLUMN_IN[0],
  holds: [{ lock: "read", from: CHECK_GO, to: CHECK_DONE, x0: 540, x1: 540 + READ_W }],
};
const FIX: Column = {
  x: 1020,
  title: "hk fix",
  color: PALETTE.warm,
  lock: { x: 1330, y: 308 },
  labelRight: 1364,
  x0: 1400,
  x1: 1760,
  at: COLUMN_IN[1],
  holds: [
    { lock: "read", from: FIX_GO, to: FIX_PATCH, x0: 1400, x1: 1400 + READ_W },
    // Handed on from its own read, 8 px on, as on the lanes.
    { lock: "write", from: FIX_WRITE, to: FIX_DONE, x0: 1400 + READ_W + HANDOFF_GAP, x1: 1760 },
  ],
};
export const COLUMNS = [CHECK, FIX] as const;
/** A hold's colour and its pill's kind: cyan for a read, warm for a write. */
const HOLD_COLOR: Record<LockMode, string> = { read: PALETTE.cyan, write: PALETTE.warm };
const HOLD_KIND: Record<LockMode, BarKind> = { read: "check", write: "fix" };
const TITLE_Y = 260;
const FILE_Y = 320;
const FILE = "src/main.py";
/** The step's row: its name right-aligned to the column's gutter, and its track. */
const ROW_Y = 390;
const ROW_STEP = "ruff-format";
const ROW_SIZE = 30;
const PILL_H = 44;
const TRACK_H = 56;
/** The padlocks are drawn half as large again as the lanes', so a phone can tell them apart. */
const LOCK_SCALE = 1.5;

/**
 * The diff card under a column's step: a panel of the eight rows of ruff's
 * diff (RUFF_FORMAT_DIFF under hk check, RUFF_FORMAT_FIX_DIFF under hk
 * fix), removals red on a red band and additions green on a green one, in
 * mono 28 on 34 px rows, x 300–900 under hk check and 1160–1760 under hk
 * fix, y 432–732: clear of the captions' band.
 */
export const DIFF = { dx: 140, y: 432, w: 600, pad: 14, rowH: 34, size: 28, inset: 20 } as const;
/** The card's top left in a column. */
const diffAt = (c: Column): Pt => ({ x: c.x + DIFF.dx, y: DIFF.y });
/** Each column's diff. */
const DIFFS = { check: RUFF_FORMAT_DIFF, fix: RUFF_FORMAT_FIX_DIFF } as const;
/** What a row of a diff is: a removal, an addition, or context. */
const diffKind = (line: string): "del" | "add" | "ctx" => (line.startsWith("-") ? "del" : line.startsWith("+") ? "add" : "ctx");
const DIFF_COLOR = { del: TERM.red, add: TERM.green, ctx: PALETTE.text2 } as const;

/** A padlock snaps over a 32nd. */
const SNAP = 1 / 8;

// Curves.

type Cubic = readonly [Pt, Pt, Pt, Pt];

function cubicAt([a, c1, c2, d]: Cubic, u: number): Pt {
  const v = 1 - u;
  return {
    x: v * v * v * a.x + 3 * v * v * u * c1.x + 3 * v * u * u * c2.x + u * u * u * d.x,
    y: v * v * v * a.y + 3 * v * v * u * c1.y + 3 * v * u * u * c2.y + u * u * u * d.y,
  };
}

/** A connector from the chip to a panel's top centre: leaving and arriving upright. */
function connector(to: Pt): Cubic {
  const dy = to.y - HUB.y;
  return [HUB, { x: HUB.x, y: HUB.y + 0.7 * dy }, { x: to.x, y: to.y - 0.7 * dy }, to];
}

function strokeCubic(ctx: CanvasRenderingContext2D, k: Cubic, u0: number, u1: number): void {
  if (u1 <= u0) return;
  const n = 32;
  ctx.beginPath();
  for (let i = 0; i <= n; i++) {
    const p = cubicAt(k, lerp(u0, u1, i / n));
    if (i === 0) ctx.moveTo(p.x, p.y);
    else ctx.lineTo(p.x, p.y);
  }
  ctx.stroke();
}

/** A pulse on a connector: a bright head with a fading tail, its glow ahead of it. */
function drawPulse(ctx: CanvasRenderingContext2D, k: Cubic, u: number, a: number): void {
  if (u <= 0 || u >= 1 || a <= 0) return;
  const head = cubicAt(k, u);
  ctx.save();
  ctx.globalAlpha *= a;
  glow(ctx, head.x, head.y, 44, PALETTE.cyan, 0.8);
  ctx.lineCap = "round";
  const n = 10;
  for (let i = n; i >= 1; i--) {
    const p0 = cubicAt(k, Math.max(0, u - (0.22 * i) / n));
    const p1 = cubicAt(k, Math.max(0, u - (0.22 * (i - 1)) / n));
    ctx.strokeStyle = rgba(PALETTE.cyanBright, 0.9 * (1 - i / (n + 1)));
    ctx.lineWidth = 7 * (1 - i / (n + 1));
    ctx.beginPath();
    ctx.moveTo(p0.x, p0.y);
    ctx.lineTo(p1.x, p1.y);
    ctx.stroke();
  }
  ctx.fillStyle = PALETTE.glint;
  ctx.beginPath();
  ctx.arc(head.x, head.y, 5, 0, TAU);
  ctx.fill();
  ctx.restore();
}

// The panels.

/**
 * A slam's squash: 1 on the impact, a rebound through a stretch, exactly 0
 * from SETTLE beats on.
 */
const SETTLE = 3 / 4;
function squash(d: number): number {
  if (d < 0 || d >= SETTLE) return 0;
  const p = d / SETTLE;
  return Math.cos(TAU * 5 * b(d)) * (1 - p) ** 2;
}

/** How far a panel leans in its flight, radians about its centre (the outer two lean into their throw). */
const flightLean = (i: number, u: number): number => (i - 1) * 3 * DEG * Math.sin(Math.PI * u);

/**
 * A landed panel's pose `since` beats after its impact: squashed by `q`
 * about its foot (x × (1 + 0.025q), y × (1 − 0.05q)) and rocked by `rock`
 * radians about its foot's centre, the outer two once, both still from
 * SETTLE on.
 */
function landedPose(i: number, since: number): { q: number; rock: number } {
  const rock = since >= 0 && since < SETTLE ? (i - 1) * 1.2 * DEG * Math.sin(TAU * 4 * b(since)) * (1 - since / SETTLE) ** 2 : 0;
  return { q: squash(since), rock };
}

interface PanelState {
  rect: Rect;
  /** 0..1 through the flight; 1 once landed. */
  fly: number;
  /** Beats since the impact (negative in flight). */
  since: number;
  /** 0..1 rolled up. */
  fold: number;
}

function panelState(i: number, bt: number): PanelState | null {
  const launch = LAUNCH[i];
  if (bt <= launch) return null;
  const fold = ROLL_UP(progress(FOLD[i], FOLD[i] + FOLD_LEN, bt));
  if (fold >= 1) return null;
  const fly = progress(launch, IMPACT[i], bt);
  const target = panelRect(i);
  // Out of the chip, accelerating into the floor: a slam.
  const rect = fly < 1 ? lerpRect(SLIVER, target, inQuad(fly)) : { ...target, h: target.h * (1 - fold) };
  return { rect, fly, since: bt - IMPACT[i], fold };
}

/**
 * The panel's box: the terminal window, a header strip over its label (in
 * proportion while it flies, fixed once it has landed and rolls up), and
 * its edge lit by `lit`.
 */
function drawBox(ctx: CanvasRenderingContext2D, r: Rect, lit: number, shadow: number, flying = false): void {
  if (r.w <= 0 || r.h <= 0) return;
  if (shadow > 0) {
    // A soft drop shadow, in device pixels (canvas shadows ignore the transform).
    const k = ctx.getTransform().a;
    ctx.save();
    ctx.shadowColor = `rgba(0,0,0,${0.55 * shadow})`;
    ctx.shadowBlur = 26 * k;
    ctx.shadowOffsetY = 10 * k;
    roundedRect(ctx, r.x, r.y, r.w, r.h, 12);
    ctx.fillStyle = TERM.window;
    ctx.fill();
    ctx.restore();
  }
  drawWindow(ctx, r, false);
  // The header strip, in the chrome's colour, over the label and command.
  const head = Math.min(r.h, flying ? ((BODY_Y - PANEL_Y) / PANEL_H) * r.h : BODY_Y - PANEL_Y);
  ctx.save();
  roundedRect(ctx, r.x, r.y, r.w, r.h, 12);
  ctx.clip();
  ctx.fillStyle = TERM.chrome;
  ctx.fillRect(r.x, r.y, r.w, head);
  ctx.fillStyle = TERM.edge;
  ctx.fillRect(r.x, r.y + head - 1, r.w, 1);
  ctx.restore();
  roundedRect(ctx, r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1, 11.5);
  ctx.strokeStyle = TERM.edge;
  ctx.lineWidth = 1;
  ctx.stroke();
  if (lit > 0) {
    roundedRect(ctx, r.x + 1, r.y + 1, r.w - 2, r.h - 2, 11);
    ctx.strokeStyle = rgba(PALETTE.cyan, 0.8 * clamp(lit));
    ctx.lineWidth = 2;
    ctx.stroke();
  }
}

/** When each pulse reaches the panels. */
const ARRIVALS = PULSES.map((p) => p + PULSE_FLY);

/** How lit a panel's edge is from the pulses reaching it. */
function edgeLit(bt: number): number {
  let lit = 0;
  for (const a of ARRIVALS) lit = Math.max(lit, bt >= a ? Math.exp(-(bt - a) / 0.18) * (1 - progress(a + 0.6, a + 0.9, bt)) : 0);
  return lit;
}

/** A panel's label, command and rows, as they arrive after its impact. */
function drawPanelText(ctx: CanvasRenderingContext2D, i: number, bt: number, lt: number, t: number, body: Rect): void {
  const p = PANELS[i];
  const imp = IMPACT[i];
  const tx = p.x + 18;
  drawWords(ctx, p.label, tx, LABEL_Y, LABEL, lt, b(imp + 1 / 8));
  const n = typedChars(p.cmd, bt, imp + 1 / 8, 1 / 4);
  if (n > 0) drawMono(ctx, p.cmd.slice(0, n), tx, CMD_Y, CMD_SIZE, PALETTE.warm);

  // The run's screen: its rows tick on one per 32nd, sliding in from the left.
  const m = mini(p.x);
  const adv = advance(m.size);
  ctx.save();
  ctx.beginPath();
  ctx.rect(body.x, body.y, body.w, body.h);
  ctx.clip();
  const fade = [p.x + PANEL_W - m.fade, p.x + PANEL_W] as const;
  p.screen.slice(0, m.rows).forEach((line, row) => {
    const at = rowStart(i, row);
    const lands = rowLands(i, row);
    const k = progress(at, lands, bt);
    if (k <= 0) return;
    const y = m.baseline0 + row * m.lineH;
    ctx.save();
    ctx.globalAlpha *= k;
    drawTermLine(ctx, line, m.x0 - 16 * (1 - swiftOut(k)), y, m.size, { t, fade });
    ctx.restore();
    // Each ✔ lands with a small green flare, rising as its row slides in and
    // peaking as it lands (TICKS in everywhere-timing.ts: the score's plucks).
    if (line.startsWith("✔")) {
      const flare = bt < lands ? k * k : Math.exp(-(bt - lands) / 0.1) * (1 - progress(lands + 0.4, lands + 0.6, bt));
      glow(ctx, m.x0 + adv / 2, y - 0.3 * m.size, 30, TERM.green, 0.7 * flare);
    }
  });
  // After each pulse arrives, a faint light runs down the rows.
  for (const a of ARRIVALS) {
    const q = progress(a, a + 1 / 2, bt);
    if (q <= 0 || q >= 1) continue;
    const y = lerp(BODY_Y - 40, FLOOR_Y + 40, q);
    const g = ctx.createLinearGradient(0, y - 40, 0, y + 40);
    g.addColorStop(0, rgba(PALETTE.cyan, 0));
    g.addColorStop(0.5, rgba(PALETTE.cyan, 0.09 * Math.sin(Math.PI * q)));
    g.addColorStop(1, rgba(PALETTE.cyan, 0));
    ctx.save();
    ctx.globalCompositeOperation = "lighter";
    ctx.fillStyle = g;
    ctx.fillRect(body.x, y - 40, body.w, 80);
    ctx.restore();
  }
  ctx.restore();
}

/** Panel `i` at beat `bt`: in flight out of the chip, slammed down, or rolling up. */
function drawPanel(ctx: CanvasRenderingContext2D, i: number, bt: number, lt: number, t: number): void {
  const s = panelState(i, bt);
  if (!s) return;
  // The outer panels lean into their throw, and rock once as they land.
  if (s.fly < 1) {
    // In flight: two ghosts behind it along its path.
    const boxAt = (u: number, a: number, lit: number) => {
      const r = lerpRect(SLIVER, panelRect(i), inQuad(u));
      ctx.save();
      ctx.globalAlpha *= a;
      ctx.translate(r.x + r.w / 2, r.y + r.h / 2);
      ctx.rotate(flightLean(i, u));
      ctx.translate(-(r.x + r.w / 2), -(r.y + r.h / 2));
      drawBox(ctx, r, lit, a < 1 ? 0 : u, true);
      ctx.restore();
    };
    for (const [lag, a] of [
      [0.3, 0.14],
      [0.15, 0.28],
    ] as const) {
      if (s.fly - lag > 0) boxAt(s.fly - lag, a, 0);
    }
    boxAt(s.fly, 1, 0.6 * s.fly);
    return;
  }
  const r = s.rect;
  const { q, rock } = landedPose(i, s.since);
  ctx.save();
  // Squashed about its foot on the impact, then a rebound, then still.
  const cx = r.x + r.w / 2;
  ctx.translate(cx, FLOOR_Y);
  if (rock) ctx.rotate(rock);
  ctx.scale(1 + 0.025 * q, 1 - 0.05 * q);
  ctx.translate(-cx, -FLOOR_Y);
  const impactLit = Math.exp(-s.since / 0.12);
  drawBox(ctx, r, Math.max(impactLit, edgeLit(bt)) * (1 - s.fold), 1 - s.fold);
  // A slam: a light spreading along the floor from its foot.
  if (s.since < 1 / 2) {
    const k = s.since / (1 / 2);
    const w = r.w * (0.9 + 0.5 * outCubic(k));
    const g = ctx.createLinearGradient(cx - w / 2, 0, cx + w / 2, 0);
    const a = 0.55 * (1 - k) ** 2;
    g.addColorStop(0, rgba(PALETTE.cyan, 0));
    g.addColorStop(0.5, rgba(PALETTE.cyanBright, a));
    g.addColorStop(1, rgba(PALETTE.cyan, 0));
    ctx.save();
    ctx.globalCompositeOperation = "lighter";
    ctx.fillStyle = g;
    ctx.fillRect(cx - w / 2, FLOOR_Y - 2, w, 3);
    ctx.restore();
    // Sparks kicked out from its feet, streaking along the floor.
    ctx.save();
    ctx.lineCap = "round";
    ctx.lineWidth = 2;
    for (let j = 0; j < 10; j++) {
      const dir = j % 2 ? 1 : -1;
      const v = 0.4 + 0.6 * hash(j + 10 * i, 83);
      const e = outCubic(k);
      const x = cx + dir * (r.w / 2 - 8 + 120 * v * e);
      const y = FLOOR_Y - 3 - 30 * v * Math.sin(Math.PI * Math.min(1, e * 1.1)) * (0.5 + 0.5 * hash(j, 89));
      const len = 14 * v * (1 - k);
      ctx.strokeStyle = rgba(PALETTE.cyanBright, 0.75 * (1 - k) ** 1.5);
      ctx.beginPath();
      ctx.moveTo(x, y);
      ctx.lineTo(x - dir * len, y + 0.3 * len);
      ctx.stroke();
    }
    ctx.restore();
  }
  // Its text, clipped to the box as it rolls up.
  ctx.save();
  roundedRect(ctx, r.x, r.y, r.w, r.h, 12);
  ctx.clip();
  ctx.globalAlpha *= 1 - smoothstep(0.4, 0.9, s.fold);
  drawPanelText(ctx, i, bt, lt, t, { x: r.x, y: BODY_Y, w: r.w, h: FLOOR_Y - BODY_Y });
  ctx.restore();
  ctx.restore();
}

/**
 * Where the connector meets panel `i`: its top centre, carried through the
 * panel's lean in flight, and its squash and rock as it lands.
 */
function panelTop(i: number, s: PanelState): Pt {
  const r = s.rect;
  const cx = r.x + r.w / 2;
  if (s.fly < 1) {
    const lean = flightLean(i, s.fly);
    const cy = r.y + r.h / 2;
    return { x: cx + (r.h / 2) * Math.sin(lean), y: cy - (r.h / 2) * Math.cos(lean) };
  }
  const { q, rock } = landedPose(i, s.since);
  const h = (FLOOR_Y - r.y) * (1 - 0.05 * q);
  return { x: cx + h * Math.sin(rock), y: FLOOR_Y - h * Math.cos(rock) };
}

/** The connector to panel `i`: out with the panel, then back into the chip as it rolls up. */
function drawConnector(ctx: CanvasRenderingContext2D, i: number, bt: number): Cubic | null {
  const s = panelState(i, bt);
  if (!s) return null;
  const end = panelTop(i, s);
  const top = { x: end.x, y: Math.max(HUB.y, end.y) };
  if (top.y - HUB.y < 1) return null;
  const k = connector(top);
  // It retracts into the chip over the second half of the roll-up.
  const reach = 1 - smoothstep(0.35, 1, s.fold);
  ctx.save();
  ctx.strokeStyle = rgba(PALETTE.cyan, 0.5);
  ctx.lineWidth = 2;
  ctx.lineCap = "round";
  strokeCubic(ctx, k, 0, reach);
  ctx.restore();
  return reach >= 1 ? k : null;
}

/** The chip: kicked by each launch and each pulse, gone after the panels. */
function drawSource(ctx: CanvasRenderingContext2D, bt: number): void {
  const out = smoothstep(CHIP_OUT[0], CHIP_OUT[1], bt);
  if (out >= 1) return;
  let kick = 0;
  for (const l of LAUNCH) kick = Math.max(kick, bump(bt, l, 1 / 2));
  for (const p of PULSES) kick = Math.max(kick, 0.6 * bump(bt, p, 1 / 3));
  if (kick > 0) glow(ctx, HUB.x, HUB.y - 30, 170, PALETTE.cyan, 0.3 * kick);
  const scale = (1 + 0.05 * kick) * (1 - 0.1 * out);
  drawChip(ctx, { scale, alpha: 1 - out });
}

// The two columns.

/** How far column `c`'s step holds the file's lock at `bt`, 0..1, snapping in and out over a 32nd, and which hold it is. */
function holding(c: Column, bt: number): { k: number; hold: Hold } {
  let best = { k: 0, hold: c.holds[0] };
  for (const h of c.holds) {
    const k = progress(h.from, h.from + SNAP, bt) * (1 - progress(h.to, h.to + SNAP, bt));
    if (k > best.k) best = { k, hold: h };
  }
  return best;
}

/**
 * A column's padlock: read (with a pip for its one reader) or write while
 * its step holds the file, open before, between and after. `held` is how
 * far it shows its held colour, 0..1: it cross-fades from open over the
 * same 32nd as its shackle shuts and its row's node lights, and back as
 * the shackle springs open.
 */
function lockAt(c: Column, bt: number): { held: number; hold: Hold; lift: number; pips: number } {
  const { k, hold } = holding(c, bt);
  const now = c.holds.find((h) => bt >= h.from && bt < h.to);
  if (now) return { held: k, hold: now, lift: 1 - land(bt, now.from, SNAP, 0.2), pips: now.lock === "read" ? 1 : 0 };
  const last = c.holds.filter((h) => h.to <= bt).pop();
  return { held: k, hold, lift: last ? land(bt, last.to, SNAP, 0.2) : 1, pips: 0 };
}

/**
 * The line from a column's padlock to its step: out of the padlock's right
 * side, round a corner and down the gutter between the row's label and its
 * track, to a node on the row. While the step holds the lock the line and
 * the node light in its hold's colour, and the node joins the track.
 */
const BUS_Y = 322;
const busX = (c: Column): number => c.x0 - 18;
const BUS_R = 12;

function busPath(ctx: CanvasRenderingContext2D, c: Column, y1: number): void {
  const x0 = c.lock.x + 16 * LOCK_SCALE;
  const x = busX(c);
  ctx.beginPath();
  ctx.moveTo(x0, BUS_Y);
  if (y1 <= BUS_Y + BUS_R) {
    ctx.lineTo(lerp(x0, x - BUS_R, clamp((y1 - BUS_Y) / BUS_R + 1)), BUS_Y);
    return;
  }
  ctx.arcTo(x, BUS_Y, x, y1, BUS_R);
  ctx.lineTo(x, y1);
}

function drawBus(ctx: CanvasRenderingContext2D, c: Column, bt: number, reveal: number): void {
  if (reveal <= 0) return;
  const x = busX(c);
  ctx.save();
  ctx.lineCap = "round";
  ctx.lineWidth = 2;
  ctx.strokeStyle = rgba(PALETTE.text3, 0.45);
  const drawn = lerp(BUS_Y - BUS_R, ROW_Y, swiftOut(reveal));
  busPath(ctx, c, drawn);
  ctx.stroke();
  const { k: h, hold } = holding(c, bt);
  const color = HOLD_COLOR[hold.lock];
  if (h > 0) {
    // Never lit further than the line has drawn in.
    ctx.strokeStyle = rgba(color, 0.9 * h);
    ctx.lineWidth = 3;
    busPath(ctx, c, Math.min(lerp(BUS_Y - BUS_R, ROW_Y, h), drawn));
    ctx.stroke();
  }
  // The row's node: a ring, filled and glowing while the step holds the lock.
  const k = progress(0.5, 0.7, reveal);
  if (k > 0) {
    ctx.fillStyle = PALETTE.bg;
    ctx.strokeStyle = rgba(h > 0 ? color : PALETTE.text3, h > 0 ? 1 : 0.7 * k);
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(x, ROW_Y, 5.5 * k, 0, TAU);
    ctx.fill();
    ctx.stroke();
    if (h > 0) {
      ctx.strokeStyle = rgba(color, 0.9 * h);
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(x, ROW_Y);
      ctx.lineTo(lerp(x, c.x0, h), ROW_Y);
      ctx.stroke();
      glow(ctx, x, ROW_Y, 34, color, 0.55 * h);
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(x, ROW_Y, 5.5 * (0.4 + 0.6 * land(h, 0, 1, 0.5)), 0, TAU);
      ctx.fill();
    }
  }
  ctx.restore();
}

/** A column's frame: title, file, padlock and its line, the step's label and track, drawn in from its `at`. */
function drawColumnFrame(ctx: CanvasRenderingContext2D, c: Column, bt: number, lt: number): void {
  drawWords(ctx, c.title, c.x, TITLE_Y, wordStyle(56, c.color), lt, b(c.at + 1 / 4));
  const n = typedChars(FILE, bt, c.at + 1 / 8, 1 / 4);
  if (n > 0) drawMono(ctx, FILE.slice(0, n), c.x, FILE_Y, 40, PALETTE.text2);
  const k = swiftOut(progress(c.at, c.at + 3 / 8, bt));
  if (k > 0) {
    ctx.save();
    ctx.globalAlpha *= k;
    drawMono(ctx, ROW_STEP, c.labelRight - monoWidth(ROW_STEP, ROW_SIZE) - 12 * (1 - k), ROW_Y + 0.3 * ROW_SIZE, ROW_SIZE, PALETTE.text2);
    ctx.restore();
    roundedRect(ctx, c.x0, ROW_Y - TRACK_H / 2, (c.x1 - c.x0) * k, TRACK_H, 10);
    ctx.fillStyle = PALETTE.elevated;
    ctx.fill();
  }
  drawBus(ctx, c, bt, progress(c.at + 1 / 4, c.at + COLUMN_DRAW, bt));
  // The padlock pops in, then shows who holds the file.
  const pop = land(bt, c.at + 1 / 4, COLUMN_DRAW - 1 / 4, 0.3);
  if (pop > 0) {
    const { x, y } = c.lock;
    const lock = lockAt(c, bt);
    const open = { lift: lock.lift, scale: LOCK_SCALE * pop };
    const held = { ...open, alpha: lock.held, pips: lock.pips };
    if (lock.held <= 0) drawPadlock(ctx, x, y, "open", open);
    else if (lock.held >= 1) drawPadlock(ctx, x, y, lock.hold.lock, held);
    else {
      // Cross-fading: the held padlock over the open one at `held`, on one
      // shackle. As the shackle snaps shut it overshoots into the body,
      // where only an opaque fill hides it, so both are drawn round the
      // body's inside (kit/lanes.ts: a 32×24 body at y −3, 3 px stroke),
      // which then takes the held fill at `held`.
      const s = open.scale;
      const inside = () => roundedRect(ctx, x - 14 * s, y - s, 28 * s, 20 * s, 3 * s);
      ctx.save();
      inside();
      ctx.rect(x - 120, y - 140, 240, 240);
      ctx.clip("evenodd");
      drawPadlock(ctx, x, y, "open", open);
      drawPadlock(ctx, x, y, lock.hold.lock, held);
      ctx.restore();
      ctx.save();
      ctx.globalAlpha *= lock.held;
      ctx.fillStyle = BAR_FILL[HOLD_KIND[lock.hold.lock]];
      inside();
      ctx.fill();
      ctx.restore();
    }
  }
}

/** How lit a running pill's leading edge is: up over a 32nd from its start, out over a 16th from its end. */
const tipLit = (bt: number, start: number, end: number): number => progress(start, start + 1 / 8, bt) * (1 - progress(end, end + 1 / 4, bt));

/**
 * A pill only a few radii wide (the lanes' bars' 8 px) is a tall thin
 * capsule, a "|", so it fades in as it widens from one radius to four,
 * out of its lit leading edge, and reads as a bar growing from its start.
 */
const PILL_IN = [LANES.barRadius, 4 * LANES.barRadius] as const;

/**
 * The step's pills at `bt`: each hold's from its start, growing to its end
 * (a read quickly, as its command's output comes in; a write steadily, as
 * hk applies the patch), its leading edge lit while it runs, and the step's
 * mark at the end of the last once it is done: hk's ✔, or its ✗ in red.
 */
function drawPills(ctx: CanvasRenderingContext2D, c: Column, bt: number, mark: { fail: boolean; flourish: number }): void {
  for (const h of c.holds) {
    if (bt < h.from) break;
    const u = progress(h.from, h.to, bt);
    const x1 = lerp(h.x0, h.x1, h.lock === "read" ? swiftOut(u) : u);
    drawBar(ctx, { x0: h.x0, x1, y0: ROW_Y - PILL_H / 2, y1: ROW_Y + PILL_H / 2 }, HOLD_KIND[h.lock], { alpha: smoothstep(PILL_IN[0], PILL_IN[1], x1 - h.x0) });
    const tip = tipLit(bt, h.from, h.to);
    if (tip > 0) glow(ctx, x1, ROW_Y, 46, HOLD_COLOR[h.lock], 0.45 * tip);
  }
  const last = c.holds[c.holds.length - 1];
  const k = land(bt, last.to, 1 / 4, 0.3);
  if (k <= 0) return;
  const cx = last.x1 - 22;
  const color = mark.fail ? TERM.red : PALETTE.green;
  if (mark.flourish > 0) glow(ctx, cx, ROW_Y, 40, color, 0.5 * mark.flourish);
  const size = 30 * k * (1 + 0.18 * mark.flourish);
  if (mark.fail) drawCross(ctx, cx, ROW_Y, 0.62 * size, color);
  else drawDone(ctx, cx, ROW_Y, { size });
}

/** A diff row's state as hk applies the patch. */
interface DiffRow {
  /** Struck through, left to right, 0..1: a removal being applied. */
  strike: number;
  /** Folded away, 0..1: its height goes as the removal is made. */
  fold: number;
  /** Settled into the file, 0..1: an addition's + and band go, and its text steps a column left. */
  settle: number;
}
const AS_PRINTED: DiffRow = { strike: 0, fold: 0, settle: 0 };

/**
 * The card of diff `lines` with its top left at `at`: `shown` of its eight
 * rows (fractional, so it grows row by row, each sliding in from the left),
 * each in its state, its edge lit by `lit` in `edge`. Its height follows
 * the rows it shows, less the ones folded away.
 */
function drawDiffCard(
  ctx: CanvasRenderingContext2D,
  at: Pt,
  lines: readonly string[],
  o: { shown?: number; rows?: (r: number) => DiffRow; alpha?: number; lit?: number; edge?: string; scale?: number },
): void {
  const a = o.alpha ?? 1;
  const shown = o.shown ?? lines.length;
  if (a <= 0 || shown <= 0) return;
  const rows = lines.map((_, r) => ({ reveal: clamp(shown - r), ...(o.rows?.(r) ?? AS_PRINTED) }));
  const tall = rows.map((s) => s.reveal * (1 - s.fold) * DIFF.rowH);
  const h = 2 * DIFF.pad * Math.min(1, 2 * shown) + tall.reduce((sum, t) => sum + t, 0);
  const rect = { x: at.x, y: at.y, w: DIFF.w, h };
  ctx.save();
  ctx.globalAlpha *= a;
  if (o.scale !== undefined && o.scale !== 1) {
    ctx.translate(at.x + DIFF.w / 2, at.y + h / 2);
    ctx.scale(o.scale, o.scale);
    ctx.translate(-(at.x + DIFF.w / 2), -(at.y + h / 2));
  }
  drawCardPanel(ctx, rect, { radius: 12, fill: TERM.window, stroke: TERM.edge });
  if ((o.lit ?? 0) > 0) {
    roundedRect(ctx, rect.x + 1, rect.y + 1, rect.w - 2, rect.h - 2, 11);
    ctx.strokeStyle = rgba(o.edge ?? PALETTE.cyan, 0.8 * clamp(o.lit ?? 0));
    ctx.lineWidth = 2;
    ctx.stroke();
  }
  roundedRect(ctx, rect.x, rect.y, rect.w, rect.h, 12);
  ctx.clip();
  const adv = 0.6 * DIFF.size;
  let y = at.y + DIFF.pad;
  lines.forEach((line, r) => {
    const s = rows[r];
    const kind = diffKind(line);
    const rowH = tall[r];
    if (s.reveal <= 0) return;
    const color = DIFF_COLOR[kind];
    // The band behind a removal or an addition: it goes as the row folds or settles.
    const band = kind === "ctx" ? 0 : 0.14 * (1 - s.settle);
    if (band > 0 && rowH > 0.5) {
      ctx.fillStyle = rgba(color, band);
      ctx.fillRect(rect.x + 8, y, rect.w - 16, rowH);
    }
    const fade = s.reveal * (1 - smoothstep(0, 0.6, s.fold));
    if (fade > 0) {
      ctx.save();
      ctx.globalAlpha *= fade;
      const x0 = rect.x + DIFF.inset - 16 * (1 - swiftOut(s.reveal)) - adv * s.settle;
      const base = y + 0.72 * DIFF.rowH;
      // Column 0 is diff's marker: it goes as the row settles into the
      // file, and the text steps into its place.
      if (line) drawMono(ctx, line[0], x0, base, DIFF.size, rgba(color, 1 - s.settle));
      drawMono(ctx, line.slice(1), x0 + adv, base, DIFF.size, kind === "del" ? color : mix(color, PALETTE.text1, s.settle));
      if (s.strike > 0) {
        ctx.strokeStyle = TERM.red;
        ctx.lineWidth = 2.5;
        ctx.lineCap = "round";
        ctx.beginPath();
        ctx.moveTo(x0, base - 0.3 * DIFF.size);
        ctx.lineTo(x0 + monoWidth(line, DIFF.size) * s.strike, base - 0.3 * DIFF.size);
        ctx.stroke();
      }
      ctx.restore();
    }
    y += rowH;
  });
  ctx.restore();
}

/** Each row of hk fix's diff as hk applies it: removals struck one after another and folded away, then the additions settle. */
function applied(bt: number): (r: number) => DiffRow {
  const a = progress(APPLY[0], APPLY[1], bt);
  const kinds = DIFFS.fix.map(diffKind);
  const dels = kinds.flatMap((k, r) => (k === "del" ? [r] : []));
  return (r) => {
    const kind = kinds[r];
    if (kind === "del") {
      const k = dels.indexOf(r);
      return { strike: smoothstep(0.08 * k, 0.08 * k + 0.3, a), fold: smoothstep(0.45, 0.85, a), settle: 0 };
    }
    return { strike: 0, fold: 0, settle: smoothstep(0.55, 1, a) };
  };
}

/** hk check: its step reads the file, fails on the diff, and the diff unfolds under it. */
function drawCheck(ctx: CanvasRenderingContext2D, bt: number): void {
  if (bt < CHECK_GO) return;
  drawPills(ctx, CHECK, bt, { fail: true, flourish: bump(bt, CHECK_DONE, 1 / 2) });
  // The failure lights the card's edge red as it unfolds; it dims a little
  // as hk fix's own diff comes up.
  const n = DIFFS.check.length;
  const shown = progress(DIFF_IN, DIFF_IN + n * DIFF_ROW, bt) * n;
  const lit = Math.exp(-Math.max(0, bt - DIFF_IN) / 0.25) * (1 - progress(FIX_GO, FIX_GO + 1 / 4, bt));
  drawDiffCard(ctx, diffAt(CHECK), DIFFS.check, { shown, alpha: 1 - 0.3 * smoothstep(FIX_PATCH, FIX_WRITE, bt), lit, edge: TERM.red });
}

/**
 * hk fix: the same command reads the file, which ruff has fixed first, and
 * its diff comes up under it; the step trades its read lock for the write
 * lock, hk applies the diff, and the step's ✔.
 */
function drawFix(ctx: CanvasRenderingContext2D, bt: number): void {
  if (bt < FIX_GO) return;
  drawPills(ctx, FIX, bt, { fail: false, flourish: bump(bt, ALL_DONE, 1 / 2) });
  if (bt >= FIX_PATCH) {
    const n = DIFFS.fix.length;
    const shown = progress(FIX_PATCH, FIX_PATCH + n * FIX_DIFF_ROW, bt) * n;
    // Its edge comes up cyan with the diff the read printed, turns warm as
    // the write lock shuts, and fades as the patch goes in.
    const lit = Math.exp(-Math.max(0, bt - FIX_WRITE) / 0.25);
    const edge = mix(PALETTE.cyan, PALETTE.warm, smoothstep(FIX_WRITE, FIX_WRITE + SNAP, bt));
    drawDiffCard(ctx, diffAt(FIX), DIFFS.fix, { shown, rows: applied(bt), lit, edge });
  }
  // A warm flare where the step takes the write lock.
  const write = FIX.holds[1];
  if (bt >= write.from) glow(ctx, write.x0, ROW_Y, 50, PALETTE.warm, 0.5 * Math.exp(-(bt - write.from) / 0.12) * (1 - progress(write.from + 0.5, write.from + 0.75, bt)));
}

/** Both columns at beat `bt`: what whips out at the end. */
function drawColumns(ctx: CanvasRenderingContext2D, bt: number, lt: number): void {
  if (bt < COLUMN_IN[0]) return;
  for (const c of COLUMNS) drawColumnFrame(ctx, c, bt, lt);
  drawCheck(ctx, bt);
  drawFix(ctx, bt);
}

// The frame.

/** The stage's jolt from the three slams: a small dip and rebound, zero on each impact frame. */
function slamJolt(lt: number): number {
  let y = 0;
  for (const imp of IMPACT) y += 4 * jolt(lt, b(imp), b(1 / 2), 9);
  return y;
}

function draw(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  ctx.fillStyle = PALETTE.bg;
  ctx.fillRect(0, 0, env.W, env.H);
  const bt = lt / BEAT;
  // The whip (whip.ts): the columns wind up to the right, then whip out
  // to the left, smeared, the streaks over them.
  if (env.t >= WHIP_START) {
    drawWhipOut(ctx, env.t, () => drawColumns(ctx, bt, lt));
    drawWhip(ctx, env.t);
    return;
  }
  ctx.save();
  const dip = slamJolt(lt);
  if (dip) ctx.translate(0, dip);
  // Connectors under the panels and the chip; pulses over the connectors.
  const curves = PANELS.map((_, i) => drawConnector(ctx, i, bt));
  PULSES.forEach((p) => {
    const u = progress(p, p + PULSE_FLY, bt);
    for (const k of curves) if (k) drawPulse(ctx, k, u, 1);
    // Where it lands, a flare on each panel's top.
    const a = p + PULSE_FLY;
    const flare = bt >= a ? Math.exp(-(bt - a) / 0.12) * (1 - progress(a + 0.4, a + 0.6, bt)) : 0;
    if (flare > 0) for (const k of curves) if (k) glow(ctx, k[3].x, k[3].y, 60, PALETTE.cyan, 0.6 * flare);
  });
  for (let i = 0; i < PANELS.length; i++) drawPanel(ctx, i, bt, lt, env.t);
  drawSource(ctx, bt);
  ctx.restore();
  drawColumns(ctx, bt, lt);
}

/** How far the panels are up, for the vignette: in as they land, out as they roll up. */
function litAlpha(bt: number): number {
  return smoothstep(IMPACT[0], IMPACT[2] + 1 / 2, bt) * (1 - smoothstep(FOLD[0], FOLD[2] + FOLD_LEN, bt));
}

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw,
  // The three panels, while they are up (one rect round all three: a scene has one lit screen).
  lit(lt): LitRect | null {
    const a = litAlpha(lt / BEAT);
    return a > 0 ? termLit(PANELS_LIT, a) : null;
  },
  captions: () => CAPTIONS,
};
