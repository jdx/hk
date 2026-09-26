// Section 8, "Commit, terminal, CI" (storyboard §6.8): two ideas in twelve
// beats.
//
// One set of steps. Three panels slam down out of the hk.pkl chip, one per
// beat, each hanging from it on a cyan connector: the pre-commit hook
// (`git commit`), a terminal (`hk fix`) and CI (`hk check --all`). Each
// lists the top of the final screen of a real run (kit/screens.ts `final`),
// its rows ticking on in a 32nd cascade, and all three name the same seven
// steps. Pulses run down the connectors on the eighths, from the one config
// into all three.
//
// Checks share a file; fixes take turns. The panels roll up, their
// connectors retract into the chip, which goes, and two small timelines for
// one file, README.md, draw in. Under `hk check` three cyan checks leave the
// same line together, all three lit on the line down from the read lock at
// once, and finish in the order `hk check --all` finished them
// (check-all.frames.txt: newlines, trailing-whitespace, then prettier).
// Under `hk fix` three warm fixes take the write lock one per beat, in the
// commit run's order: the padlock opens and shuts between them and the lit
// node slides down to the next, and each pill grows only once the key has
// reached it. The contrast is also the score's: a three-note chord on b6.5,
// the same notes one at a time as each fix takes the lock. The beats live in
// everywhere-timing.ts, which the score can import.
//
// The section leaves on the whip into the race (whip.ts): from b10.5 the
// columns wind up, whip out to the left and the streaks take over, still
// moving on its last frame. Nothing here depends on the benchmark facts.

import { BEAT, type LitRect, PALETTE, type Scene, type SceneEnv, sec, TERM } from "../bible";
import { rgba } from "../color";
import { glow, roundedRect } from "../fx";
import { drawChip, drawMono, HKPKL_CHIP, monoWidth } from "../kit/card";
import { drawBar, drawDone, drawPadlock, type LockState } from "../kit/lanes";
import { bump, jolt, land, lerpRect, type Pt, type Rect, typedChars } from "../kit/motion";
import { final, type Screen } from "../kit/screens";
import { advance, drawTermLine, drawWindow, mini, termLit } from "../kit/term";
import { clamp, DEG, hash, inQuad, keys, lerp, outCubic, outQuad, progress, smoothstep, swiftInOut, swiftOut, TAU } from "../math";
import { type Caption, drawWords, LABEL, wordStyle } from "../type";
import { drawWhip, drawWhipOut, WHIP_START } from "../whip";
import {
  ALL_DONE,
  CHECK_DONE,
  CHECK_GO,
  CHIP_OUT,
  COLUMN_IN,
  FIX_HOLDS,
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
  // 4 + 3 words: need 4.5, hold 4.5.
  { out: 6, lines: [{ in: 1.5, text: "One set of steps:" }, { in: 2.5, text: "commit, terminal, CI." }] },
  // 4 + 3 words: need 4.5, hold 4.5; its first word rises on b6, as the caption above starts to leave.
  { out: 11, lines: [{ in: 6.5, text: "Checks share a file." }, { in: 7, text: "Fixes take turns." }] },
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

/** The three panels (storyboard §6.8): a label, the command, and the top 9 rows of its run's last screen. */
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

interface Column {
  x: number;
  title: string;
  color: string;
  kind: "check" | "fix";
  lock: Pt;
  lockState: LockState;
  labelRight: number;
  x0: number;
  x1: number;
  at: number;
}

/** The two timelines for one file (storyboard §6.8). */
const CHECK: Column = { x: 160, title: "hk check", color: PALETTE.cyan, kind: "check", lock: { x: 470, y: 338 }, lockState: "read", labelRight: 504, x0: 540, x1: 900, at: COLUMN_IN[0] };
const FIX: Column = { x: 1020, title: "hk fix", color: PALETTE.warm, kind: "fix", lock: { x: 1330, y: 338 }, lockState: "write", labelRight: 1364, x0: 1400, x1: 1760, at: COLUMN_IN[1] };
const COLUMNS = [CHECK, FIX] as const;
const TITLE_Y = 290;
const FILE_Y = 350;
const FILE = "README.md";
const ROW_Y = [440, 520, 600] as const;
const ROW_STEPS = ["prettier", "trailing-whitespace", "newlines"] as const;
const ROW_SIZE = 30;
const PILL_H = 44;
const TRACK_H = 56;
/** The padlocks are drawn half as large again as the lanes', so a phone can tell them apart. */
const LOCK_SCALE = 1.5;

/**
 * The checks grow together at one rate, a shared clock, so a shorter bar is
 * a read that ended sooner: prettier to x 900 on b8, and the others where
 * the clock stands when their reads end (trailing-whitespace x 720 on
 * b7.25, newlines x 660 on b7).
 */
const CHECK_RATE = (900 - CHECK.x0) / (CHECK_DONE[0] - CHECK_GO);
const CHECK_END = CHECK_DONE.map((d) => CHECK.x0 + CHECK_RATE * (d - CHECK_GO));

/** The fixes, one after another, 8 px apart where the lock is handed on (as the lanes); each grows while it holds the lock. */
const FIX_PILLS = [
  { x0: 1400, x1: 1560 },
  { x0: 1568, x1: 1660 },
  { x0: 1668, x1: 1760 },
] as const;

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

/**
 * When each row holds its column's lock, in beats: the checks all from the
 * go until their reads end; the fixes one after another, each taking the
 * lock a 16th after the last let it go (the lanes' rule, lanes.ts).
 */
const HOLDS: Record<Column["kind"], readonly (readonly [from: number, to: number])[]> = {
  check: CHECK_DONE.map((d) => [CHECK_GO, d] as const),
  fix: FIX_HOLDS,
};

/** How far row `r` holds column `c`'s lock at `bt`, 0..1, snapping in and out over a 32nd. */
function holding(c: Column, r: number, bt: number): number {
  const [from, to] = HOLDS[c.kind][r];
  return progress(from, from + SNAP, bt) * (1 - progress(to, to + SNAP, bt));
}

/** A column's padlock: read or write while any row holds it, open between and after. */
function lockAt(c: Column, bt: number): { state: LockState; lift: number } {
  const holds = HOLDS[c.kind];
  const held = holds.filter(([from, to]) => bt >= from && bt < to);
  if (held.length) return { state: c.lockState, lift: 1 - land(bt, Math.min(...held.map(([from]) => from)), SNAP, 0.2) };
  const released = holds.filter(([, to]) => to <= bt).reduce((m, [, to]) => Math.max(m, to), -Infinity);
  return { state: "open", lift: released === -Infinity ? 1 : land(bt, released, SNAP, 0.2) };
}

/**
 * The reader line down from a column's padlock: out of its right side,
 * round a corner and down the gutter between the row labels and the
 * tracks, with a node at each row. A row that holds the lock lights its
 * node and the line from the padlock down to it: under hk check all three
 * at once (the read lock's three pips), under hk fix one at a time, the
 * key sliding down to the next fix while the lock is open.
 */
const BUS_Y = 352;
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

/** How far down the bus is lit (its y), and how brightly. */
function busLit(c: Column, bt: number): { y: number; a: number; key: number | null } {
  if (c.kind === "check") {
    // Lit to the lowest row still reading, retracting as reads end.
    let y = BUS_Y - BUS_R;
    let a = 0;
    ROW_Y.forEach((cy, r) => {
      const h = holding(c, r, bt);
      if (h <= 0) return;
      y = Math.max(y, lerp(r ? ROW_Y[r - 1] : BUS_Y - BUS_R, cy, h));
      a = Math.max(a, h);
    });
    return { y, a, key: null };
  }
  // Unlit until the first fix takes the lock; then the fix holding it, or
  // the key on its way down to the next.
  if (bt < HOLDS.fix[0][0]) return { y: BUS_Y - BUS_R, a: 0, key: null };
  for (let r = 0; r < ROW_Y.length; r++) {
    const [from, to] = HOLDS.fix[r];
    if (r === 0 && bt < from + SNAP) return { y: lerp(BUS_Y - BUS_R, ROW_Y[0], progress(from, from + SNAP, bt)), a: 1, key: null };
    if (bt >= from && bt < to) return { y: ROW_Y[r], a: 1, key: null };
    const next = HOLDS.fix[r + 1];
    if (next && bt >= to && bt < next[0]) {
      const y = lerp(ROW_Y[r], ROW_Y[r + 1], swiftInOut(progress(to, next[0], bt)));
      return { y, a: 1, key: y };
    }
  }
  const last = HOLDS.fix[ROW_Y.length - 1][1];
  return { y: ROW_Y[ROW_Y.length - 1], a: 1 - progress(last, last + 2 * SNAP, bt), key: null };
}

function drawBus(ctx: CanvasRenderingContext2D, c: Column, bt: number, reveal: number): void {
  if (reveal <= 0) return;
  const x = busX(c);
  ctx.save();
  ctx.lineCap = "round";
  ctx.lineWidth = 2;
  ctx.strokeStyle = rgba(PALETTE.text3, 0.45);
  const drawn = lerp(BUS_Y - BUS_R, ROW_Y[2], swiftOut(reveal));
  busPath(ctx, c, drawn);
  ctx.stroke();
  const lit = busLit(c, bt);
  if (lit.a > 0) {
    // Never lit further than the line has drawn in.
    ctx.strokeStyle = rgba(c.color, 0.9 * lit.a);
    ctx.lineWidth = 3;
    busPath(ctx, c, Math.min(lit.y, drawn));
    ctx.stroke();
  }
  // A node per row: a ring, filled and glowing while its row holds the lock.
  ROW_Y.forEach((cy, r) => {
    const k = progress(0.3 + 0.2 * r, 0.5 + 0.2 * r, reveal);
    if (k <= 0) return;
    const h = holding(c, r, bt);
    ctx.fillStyle = PALETTE.bg;
    ctx.strokeStyle = rgba(h > 0 ? c.color : PALETTE.text3, h > 0 ? 1 : 0.7 * k);
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(x, cy, 5.5 * k, 0, TAU);
    ctx.fill();
    ctx.stroke();
    if (h > 0) {
      // Joined to its row's track while it holds the lock.
      ctx.strokeStyle = rgba(c.color, 0.9 * h);
      ctx.lineWidth = 3;
      ctx.beginPath();
      ctx.moveTo(x, cy);
      ctx.lineTo(lerp(x, c.x0, h), cy);
      ctx.stroke();
      glow(ctx, x, cy, 34, c.color, 0.55 * h);
      ctx.fillStyle = c.color;
      ctx.beginPath();
      ctx.arc(x, cy, 5.5 * (0.4 + 0.6 * land(h, 0, 1, 0.5)), 0, TAU);
      ctx.fill();
    }
  });
  if (lit.key !== null) {
    glow(ctx, x, lit.key, 36, PALETTE.warm, 0.8);
    ctx.fillStyle = PALETTE.warmBright;
    ctx.beginPath();
    ctx.arc(x, lit.key, 5, 0, TAU);
    ctx.fill();
  }
  ctx.restore();
}

/** A column's frame: title, file, padlock and its line, row labels and tracks, drawn in from its `at`. */
function drawColumnFrame(ctx: CanvasRenderingContext2D, c: Column, bt: number, lt: number): void {
  drawWords(ctx, c.title, c.x, TITLE_Y, wordStyle(56, c.color), lt, b(c.at + 1 / 4));
  const n = typedChars(FILE, bt, c.at + 1 / 8, 1 / 4);
  if (n > 0) drawMono(ctx, FILE.slice(0, n), c.x, FILE_Y, 40, PALETTE.text2);
  ROW_Y.forEach((cy, r) => {
    const at = c.at + r / 16;
    const k = swiftOut(progress(at, at + 3 / 8, bt));
    if (k <= 0) return;
    const label = ROW_STEPS[r];
    ctx.save();
    ctx.globalAlpha *= k;
    drawMono(ctx, label, c.labelRight - monoWidth(label, ROW_SIZE) - 12 * (1 - k), cy + 0.3 * ROW_SIZE, ROW_SIZE, PALETTE.text2);
    ctx.restore();
    roundedRect(ctx, c.x0, cy - TRACK_H / 2, (c.x1 - c.x0) * k, TRACK_H, 10);
    ctx.fillStyle = PALETTE.elevated;
    ctx.fill();
  });
  drawBus(ctx, c, bt, progress(c.at + 1 / 4, c.at + 1 / 2, bt));
  // The padlock pops in, then shows who holds the file.
  const pop = land(bt, c.at + 1 / 4, 1 / 4, 0.3);
  if (pop > 0) {
    const lock = lockAt(c, bt);
    drawPadlock(ctx, c.lock.x, c.lock.y, lock.state, { lift: lock.lift, scale: LOCK_SCALE * pop });
  }
}

/** How lit a running pill's leading edge is: up over a 32nd from its start, out over a 16th from its end. */
const tipLit = (bt: number, start: number, end: number): number => progress(start, start + 1 / 8, bt) * (1 - progress(end, end + 1 / 4, bt));

/** One step's pill from x0 to x1 on row `r`, its leading edge lit while it runs, and its ✔ once done. */
function drawPill(ctx: CanvasRenderingContext2D, c: Column, r: number, x0: number, x1: number, tip: number, done: number, flourish: number): void {
  const cy = ROW_Y[r];
  drawBar(ctx, { x0, x1, y0: cy - PILL_H / 2, y1: cy + PILL_H / 2 }, c.kind);
  if (tip > 0) glow(ctx, x1, cy, 46, c.color, 0.45 * tip);
  if (done > 0) {
    const cx = x1 - 22;
    if (flourish > 0) glow(ctx, cx, cy, 40, PALETTE.green, 0.5 * flourish);
    drawDone(ctx, cx, cy, { size: 30, scale: done * (1 + 0.18 * flourish) });
  }
}

function drawChecks(ctx: CanvasRenderingContext2D, bt: number): void {
  if (bt < CHECK_GO) return;
  const go = bt - CHECK_GO;
  const flourish = bump(bt, ALL_DONE, 1 / 2);
  ROW_Y.forEach((_, r) => {
    const end = CHECK_DONE[r];
    const x1 = Math.min(CHECK_END[r], CHECK.x0 + CHECK_RATE * go);
    drawPill(ctx, CHECK, r, CHECK.x0, x1, tipLit(bt, CHECK_GO, end), land(bt, end, 1 / 4, 0.3), flourish);
  });
  // All three leave the same line together.
  const flash = Math.exp(-go / 0.15) * (1 - progress(0.5, 0.75, go));
  if (flash > 0.001) {
    ctx.save();
    ctx.globalCompositeOperation = "lighter";
    ctx.fillStyle = rgba(PALETTE.cyanBright, 0.8 * flash);
    ctx.fillRect(CHECK.x0 - 1.5, ROW_Y[0] - TRACK_H / 2 - 6, 3, ROW_Y[2] - ROW_Y[0] + TRACK_H + 12);
    ctx.restore();
    for (const cy of ROW_Y) glow(ctx, CHECK.x0, cy, 60, PALETTE.cyan, 0.5 * flash);
  }
}

function drawFixes(ctx: CanvasRenderingContext2D, bt: number): void {
  const flourish = bump(bt, ALL_DONE, 1 / 2);
  FIX_PILLS.forEach((f, r) => {
    // It grows only while it holds the write lock: from the moment the key
    // reaches its node to its ✔ on the beat.
    const [from, to] = FIX_HOLDS[r];
    if (bt < from) return;
    const x1 = lerp(f.x0, f.x1, progress(from, to, bt));
    drawPill(ctx, FIX, r, f.x0, x1, tipLit(bt, from, to), land(bt, to, 1 / 4, 0.3), flourish);
    // It takes the lock: a warm flare where it starts.
    const flare = Math.exp(-(bt - from) / 0.12) * (1 - progress(from + 0.5, from + 0.75, bt));
    glow(ctx, f.x0, ROW_Y[r], 50, PALETTE.warm, 0.5 * flare);
  });
}

/** Both columns at beat `bt`: what whips out at the end. */
function drawColumns(ctx: CanvasRenderingContext2D, bt: number, lt: number): void {
  if (bt < COLUMN_IN[0]) return;
  for (const c of COLUMNS) drawColumnFrame(ctx, c, bt, lt);
  drawChecks(ctx, bt);
  drawFixes(ctx, bt);
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
