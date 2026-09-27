// The config scene's rivers (storyboard §6.2): today's builtin identifiers
// (kit/builtins-list.ts) streaming across the stage as three seeded, curved
// rivers at three depths. They are texture, not text to read, so they stay
// faint, above the captions' band, and every name is drawn at most once.
//
// Each river is a gentle wave across the frame, two names thick. The back
// one is small, dim, slow and a little out of focus; the front one large,
// brighter and quicker, so the three slide past each other in parallax. A
// river washes in from the left behind a soft front, and its flow eases from
// a surge to a cruise. When the card arrives the rivers swing a quarter turn
// clockwise about their middles and narrow into three columns in the left
// third, their names turned to read down the column like book spines, still
// flowing. The scene plucks its seven steps' names out of them.
//
// Everything is a pure function of the scene's local time. The layout is
// built once, from rng seeds, when the module loads.

import { BEAT, PALETTE } from "../bible";
import { makeCanvas } from "../fx";
import { clamp, cubicBezier, lerp, outCubic, progress, rng, smoothstep, TAU } from "../math";
import { BUILTINS } from "../kit/builtins-list";
import { font, MONO } from "../type";

const b = (n: number): number => n * BEAT;

/** Where a name is and which way it reads: its visual centre, and its angle in radians (clockwise). */
export interface Pose {
  x: number;
  y: number;
  a: number;
}

export interface River {
  /** 0 back, 1 mid, 2 front. */
  depth: number;
  /** Mono size and opacity. */
  size: number;
  alpha: number;
  /** Cruising flow, px/s, and the surge it eases down from. */
  speed: number;
  surge: number;
  /** The wave: y at x 960, rise per px, amplitude, wavelength, the x of a zero crossing, drift rad/s. */
  yc: number;
  tilt: number;
  amp: number;
  wave: number;
  zero: number;
  drift: number;
  /** Perpendicular offsets of its two files of names, px. */
  lanes: readonly number[];
  /** The soft front washes across over these local beats. */
  wash: readonly [number, number];
  /** The quarter turn into the left third, local beats, and the column's x. */
  turn: readonly [number, number];
  col: number;
}

/** The point on each river it turns about: the frame's middle. */
const PIVOT = 960;
/** The columns' vertical middle, between the stage's top and FLOOR. */
const COL_Y = 405;
/**
 * The quarter turn, local beats: the same for all three rivers, so they
 * stay parallel as they swing and never pass through each other (their
 * parallax comes from their flows). Each keeps its place in the stack: a
 * clockwise quarter turn takes the top river (back) to the right-hand
 * column, the middle one (front) to the middle and the bottom one (mid) to
 * the left.
 */
const TURN_BEATS = [3.5, 4.5] as const;
/** When the drift's phase is zero: the pops (b2), so the front river crosses y 430 at x 420 and 1500 then. */
const DRIFT_T0 = b(2);

export const RIVERS: readonly River[] = [
  {
    depth: 0,
    size: 28,
    alpha: 0.24,
    speed: 70,
    surge: 220,
    yc: 236,
    tilt: 0.085,
    amp: 44,
    wave: 1480,
    zero: 250,
    drift: 0.35,
    lanes: [-19, 19],
    wash: [1, 2.5],
    turn: TURN_BEATS,
    col: 468,
  },
  {
    depth: 1,
    size: 32,
    alpha: 0.36,
    speed: 105,
    surge: 260,
    yc: 588,
    tilt: -0.075,
    amp: 48,
    wave: 1840,
    zero: 1180,
    drift: -0.3,
    lanes: [-22, 22],
    wash: [0.75, 2.25],
    turn: TURN_BEATS,
    col: 238,
  },
  {
    depth: 2,
    size: 36,
    alpha: 0.58,
    speed: 150,
    surge: 300,
    yc: 430,
    tilt: 0,
    amp: -68,
    wave: 2160,
    zero: 420,
    drift: 0.22,
    lanes: [-25, 25],
    wash: [0.5, 1.75],
    turn: TURN_BEATS,
    col: 356,
  },
];

/** The turn's ease: it gathers, swings, and settles into the column. */
const TURN = cubicBezier(0.6, 0, 0.25, 1);

/** How far river `r` has turned into its column, 0..1. */
export const turnOf = (r: River, t: number): number => TURN(progress(b(r.turn[0]), b(r.turn[1]), t));

/** How far river `r` has flowed by `t`, px along it: a surge easing into the cruise. */
export function flow(r: River, t: number): number {
  const d = Math.max(0, t - b(r.wash[0]) + 0.25);
  return r.speed * t + r.surge * (1 - Math.exp(-d / 0.55));
}

/** The soft front's x in the river's own (unturned) frame. */
const frontOf = (r: River, t: number): number => lerp(-420, 2420, outCubic(progress(b(r.wash[0]), b(r.wash[1]), t)));

/** The softness of the front, px. */
const FRONT_SOFT = 260;

/**
 * Where a name centred at `s` along river `r`, `n` px to its side, is at
 * local time `t`, and which way it reads.
 */
export function riverPose(r: River, s: number, n: number, t: number): Pose {
  const k = turnOf(r, t);
  const amp = r.amp * (1 - 0.8 * k);
  const phase = r.drift * (t - DRIFT_T0);
  const w = TAU / r.wave;
  const yAt = (u: number) => r.yc + r.tilt * (u - PIVOT) + amp * Math.sin(w * (u - r.zero) + phase);
  const slope = r.tilt + amp * w * Math.cos(w * (s - r.zero) + phase);
  const phi = Math.atan(slope);
  const px = s - n * Math.sin(phi);
  const py = yAt(s) + n * Math.cos(phi);
  if (k <= 0) return { x: px, y: py, a: phi };
  const vy = yAt(PIVOT);
  const cx = lerp(PIVOT, r.col, k);
  const cy = lerp(vy, COL_Y, k);
  const th = (k * Math.PI) / 2;
  const dx = px - PIVOT;
  const dy = py - vy;
  const c = Math.cos(th);
  const sn = Math.sin(th);
  return { x: cx + dx * c - dy * sn, y: cy + dx * sn + dy * c, a: th + phi };
}

/** How far the front has uncovered a name at `s` (its unturned x), 0..1. */
export const washOf = (r: River, s: number, t: number): number => smoothstep(0, FRONT_SOFT, frontOf(r, t) - s);

// The names.

export interface RiverName {
  text: string;
  river: number;
  /** Its file's offset from the river's line, px. */
  n: number;
  /** Its centre along the river at t = 0; at t it is at s0 + flow(t). */
  s0: number;
  size: number;
  color: string;
  alpha: number;
}

/** A name the scene places itself, at its centre `s` along the river at time `at`. */
export interface Fixed {
  text: string;
  river: number;
  lane: number;
  s: number;
  at: number;
}

/** A name's width on the mono grid. */
export const nameWidth = (text: string, size: number): number => text.length * 0.6 * size;

/** How long the rivers must run: the scene's six seconds and a margin. */
const RUN = 6.5;

/**
 * Where two rivers must not print over each other, local seconds: from the
 * first wash, across the whole frame's width (the bleeds included), through
 * the turn and into the columns.
 */
const APART = { from: b(0.5), to: b(4.75), step: 1 / 60 } as const;

/** A name's box, turned as drawn, and the circle round it, for the layout's overprint check. */
interface Placed {
  river: number;
  cx: number;
  cy: number;
  rad: number;
  pts: readonly { x: number; y: number }[];
}

/** A name's box at a pose: the mono grid's width, ascenders to descenders. */
function boxAt(text: string, p: Pose, size: number, river: number): Placed {
  const hw = (text.length * 0.6 * size) / 2;
  const y0 = -0.4 * size;
  const y1 = (/[gjpqy_]/.test(text) ? 0.55 : 0.37) * size;
  const c = Math.cos(p.a);
  const s = Math.sin(p.a);
  const pts = [
    [-hw, y0],
    [hw, y0],
    [hw, y1],
    [-hw, y1],
  ].map(([u, v]) => ({ x: p.x + u * c - v * s, y: p.y + u * s + v * c }));
  return { river, cx: p.x, cy: p.y, rad: Math.hypot(hw, Math.max(-y0, y1)), pts };
}

/** Whether two turned boxes overlap by more than `graze` px (separating axes). */
function overprints(a: Placed, q: Placed, graze: number): boolean {
  if (Math.hypot(a.cx - q.cx, a.cy - q.cy) > a.rad + q.rad) return false;
  for (const e of [a.pts, q.pts]) {
    for (let i = 0; i < 2; i++) {
      const nx = -(e[i + 1].y - e[i].y);
      const ny = e[i + 1].x - e[i].x;
      const l = Math.hypot(nx, ny) || 1;
      let a0 = Infinity;
      let a1 = -Infinity;
      let q0 = Infinity;
      let q1 = -Infinity;
      for (const p of a.pts) {
        const d = (p.x * nx + p.y * ny) / l;
        a0 = Math.min(a0, d);
        a1 = Math.max(a1, d);
      }
      for (const p of q.pts) {
        const d = (p.x * nx + p.y * ny) / l;
        q0 = Math.min(q0, d);
        q1 = Math.max(q1, d);
      }
      if (a1 - graze < q0 || q1 - graze < a0) return false;
    }
  }
  return true;
}

/**
 * Lay out the rivers: each of its files filled from beyond the right edge to
 * as far upstream as it will flow in the scene, the fixed names exactly
 * where the scene asks and the pool's names, in a seeded shuffle, in the
 * gaps between them. Each pool name is used once, while they last. A pool
 * name that would print over a name of another river anywhere on the frame
 * (the rivers' waves meet in the right-hand bleed as they wash in) slides
 * upstream, a character at a time, until it is clear.
 */
export function layoutRivers(fixed: readonly Fixed[], seed = 1729): RiverName[] {
  const rand = rng(seed);
  const taken = new Set(fixed.map((f) => f.text));
  const pool = BUILTINS.filter((n) => !taken.has(n));
  for (let i = pool.length - 1; i > 0; i--) {
    const j = Math.floor(rand() * (i + 1));
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }
  let next = 0;
  const out: RiverName[] = [];
  const style = (r: River, text: string, n: number, s0: number): RiverName => {
    const u = rand();
    const color = u < 0.58 ? PALETTE.text2 : u < 0.83 ? PALETTE.cyan : PALETTE.warm;
    // A little depth within the river: nearer names a touch larger and brighter.
    const z = rand();
    return { text, river: r.depth, n, s0, size: r.size * (0.94 + 0.12 * z), color, alpha: r.alpha * (0.8 + 0.35 * z) };
  };

  // Every name placed so far, as its visible boxes at each of APART's instants.
  const samples = Math.floor((APART.to - APART.from) / APART.step) + 1;
  const seen: Placed[][] = Array.from({ length: samples }, () => []);
  const boxes = (ri: number, text: string, n: number, s0: number): (Placed | null)[] => {
    const r = RIVERS[ri];
    const size = r.size * 1.06;
    const hw = nameWidth(text, size) / 2 + size;
    const got: (Placed | null)[] = [];
    for (let i = 0; i < samples; i++) {
      const t = APART.from + i * APART.step;
      const s = s0 + flow(r, t);
      const wash = washOf(r, s, t);
      const pose = r.alpha * wash > 0.03 ? riverPose(r, s, n - 22 * (1 - outCubic(wash)), t) : null;
      got.push(pose && r.alpha * wash * maskAt(pose.y) >= 0.04 && pose.x > -hw && pose.x < 1920 + hw ? boxAt(text, pose, size, ri) : null);
    }
    return got;
  };
  const clashes = (got: (Placed | null)[]): boolean =>
    got.some((p, i) => p !== null && seen[i].some((q) => q.river !== p.river && overprints(p, q, 0)));
  const keep = (got: (Placed | null)[]) => got.forEach((p, i) => p && seen[i].push(p));
  // The fixed names first, where the scene put them.
  fixed.forEach((f) => {
    const r = RIVERS[f.river];
    keep(boxes(f.river, f.text, r.lanes[f.lane], f.s - flow(r, f.at)));
  });

  RIVERS.forEach((r, ri) => {
    r.lanes.forEach((n, li) => {
      const adv = 0.6 * r.size;
      const mine = fixed
        .filter((f) => f.river === ri && f.lane === li)
        .map((f) => ({ f, s0: f.s - flow(r, f.at), w: nameWidth(f.text, r.size * 1.06) }))
        .sort((a, c) => c.s0 - a.s0);
      for (const m of mine) out.push(style(r, m.f.text, n, m.s0));
      // Right to left, from past the frame's right edge (and whatever the
      // front has yet to bring on) to as far back as the flow will reach.
      const lo = -900 - flow(r, RUN) - 400;
      let cursor = 2900 + adv * 6 * rand() + (li ? adv * 3 : 0);
      let k = 0;
      while (cursor > lo) {
        const gap = adv * (1.2 + 1.6 * rand());
        // The next fixed name to the left of the cursor.
        while (k < mine.length && mine[k].s0 - mine[k].w / 2 > cursor) k++;
        const f = mine[k];
        const roomAt = (c: number) => (f ? c - (f.s0 + f.w / 2) - gap : Infinity);
        let placed = false;
        for (let tries = 0; tries < 6 && next < pool.length; tries++) {
          const text = pool[next];
          const w = nameWidth(text, r.size * 1.06);
          // Slid upstream past any name of another river it would print over.
          let c = cursor;
          let got = w <= roomAt(c) ? boxes(ri, text, n, c - w / 2) : null;
          let free = got !== null && !clashes(got);
          for (let slide = 0; got && !free && slide < 60; slide++) {
            c -= adv;
            got = w <= roomAt(c) ? boxes(ri, text, n, c - w / 2) : null;
            free = got !== null && !clashes(got);
          }
          if (got && free) {
            next++;
            out.push(style(r, text, n, c - w / 2));
            keep(got);
            cursor = c - w - gap;
            placed = true;
            break;
          }
          // Too long for the room before a fixed name (or with no clear
          // place before it): try a later one.
          const j = next + 1 + tries;
          if (j < pool.length) [pool[next], pool[j]] = [pool[j], pool[next]];
        }
        if (!placed) {
          if (f) {
            cursor = f.s0 - f.w / 2 - gap;
            k++;
          } else if (next >= pool.length) {
            break;
          } else {
            // Nothing clear here: leave the stretch empty.
            cursor -= 60 * adv;
          }
        }
      }
    });
  });
  return out;
}

// Drawing.

/** A name set on the mono grid, centred on the origin (its visual centre), each glyph in its column. */
export function drawName(ctx: CanvasRenderingContext2D, text: string, size: number): void {
  ctx.font = font(size, 400, MONO);
  const adv = 0.6 * size;
  const x0 = (-text.length * adv) / 2;
  const y = 0.34 * size;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch !== " ") ctx.fillText(ch, x0 + i * adv, y);
  }
}

/** One name at a pose. */
export function drawNameAt(ctx: CanvasRenderingContext2D, text: string, p: Pose, size: number, color: string, alpha: number): void {
  if (!(alpha > 0)) return;
  ctx.save();
  ctx.globalAlpha *= clamp(alpha);
  ctx.fillStyle = color;
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
  ctx.translate(p.x, p.y);
  if (p.a) ctx.rotate(p.a);
  drawName(ctx, text, size);
  ctx.restore();
}

/** Where a river name is at `t`, and how much of it shows (the front, the reveal's drop). */
export function namePose(nm: RiverName, t: number): { pose: Pose; alpha: number } {
  const r = RIVERS[nm.river];
  const s = nm.s0 + flow(r, t);
  const wash = washOf(r, s, t);
  // Each name drops the last few px into its file as the front uncovers it.
  const n = nm.n - 22 * (1 - outCubic(wash));
  return { pose: riverPose(r, s, n, t), alpha: nm.alpha * wash };
}

// Layers: the rivers are drawn into an offscreen layer, the back river blurred
// a little on a layer of its own, then faded out at the stage's top and at
// FLOOR, above the captions' band, and composited in one go.

let layer: HTMLCanvasElement | null = null;
let backLayer: HTMLCanvasElement | null = null;

function fit(c: HTMLCanvasElement | null, w: number, h: number): HTMLCanvasElement {
  const l = c ?? makeCanvas(w, h);
  if (l.width !== w || l.height !== h) {
    l.width = w;
    l.height = h;
  }
  return l;
}

function begin(l: HTMLCanvasElement, m: DOMMatrix): CanvasRenderingContext2D {
  const g = l.getContext("2d")!;
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.globalAlpha = 1;
  g.globalCompositeOperation = "source-over";
  g.filter = "none";
  g.clearRect(0, 0, l.width, l.height);
  g.setTransform(m);
  return g;
}

export interface RiverDrawOptions {
  /** Every name's opacity is multiplied by this. */
  alpha: number;
  /** Names the scene is drawing itself at `t` (popped or plucked): skipped here. */
  skip?: (nm: RiverName) => boolean;
  /** A per-name opacity, from where the name is: the clearing round a popped or plucked word. */
  clear?: (p: Pose, nm: RiverName) => number;
}

/** The back river's blur, px: a little out of focus, behind the other two. */
export const BACK_BLUR = 1.3;

/** The mask's soft edges, px: the rivers fade in below the top one and out above FLOOR. */
const MASK = { top: [96, 170], bottom: [636, 700] } as const;

/** Paint the rivers at local time `t`. */
export function drawRivers(ctx: CanvasRenderingContext2D, names: readonly RiverName[], t: number, o: RiverDrawOptions): void {
  if (!(o.alpha > 0)) return;
  const { width, height } = ctx.canvas;
  const m = ctx.getTransform();
  layer = fit(layer, width, height);
  backLayer = fit(backLayer, width, height);
  const main = begin(layer, m);
  const back = begin(backLayer, m);
  let any = false;
  let anyBack = false;
  for (const nm of names) {
    if (o.skip?.(nm)) continue;
    const { pose, alpha } = namePose(nm, t);
    let a = alpha * o.alpha;
    if (!(a > 0.003)) continue;
    // Cull what is off the stage, generously: a name turned down its column
    // is as long as it is wide.
    const half = nameWidth(nm.text, nm.size) / 2 + 40;
    if (pose.x < -half || pose.x > 1920 + half || pose.y < MASK.top[0] - half || pose.y > MASK.bottom[1] + half) continue;
    if (o.clear) a *= o.clear(pose, nm);
    const g = nm.river === 0 ? back : main;
    drawNameAt(g, nm.text, pose, nm.size, nm.color, a);
    if (nm.river === 0) anyBack = true;
    else any = true;
  }
  if (!any && !anyBack) return;
  const px = Math.abs(m.a) || 1;
  if (anyBack) {
    // The back river, a little out of focus, under the other two.
    main.save();
    main.setTransform(1, 0, 0, 1, 0, 0);
    main.globalCompositeOperation = "destination-over";
    main.filter = `blur(${(BACK_BLUR * px).toFixed(2)}px)`;
    main.drawImage(backLayer, 0, 0);
    main.restore();
  }
  // Fade out at the top and above FLOOR.
  main.save();
  main.globalCompositeOperation = "destination-in";
  const grad = main.createLinearGradient(0, MASK.top[0], 0, MASK.bottom[1]);
  const span = MASK.bottom[1] - MASK.top[0];
  const at = (y: number) => (y - MASK.top[0]) / span;
  grad.addColorStop(0, "rgba(0,0,0,0)");
  grad.addColorStop(at(MASK.top[1]), "rgba(0,0,0,1)");
  grad.addColorStop(at(MASK.bottom[0]), "rgba(0,0,0,1)");
  grad.addColorStop(1, "rgba(0,0,0,0)");
  main.fillStyle = grad;
  main.fillRect(-10, -10, 1940, 1100);
  main.restore();
  ctx.save();
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha = 1;
  ctx.drawImage(layer, 0, 0);
  ctx.restore();
}

/** The mask's opacity at `y`, for a name that leaves the rivers there. */
export const maskAt = (y: number): number => smoothstep(MASK.top[0], MASK.top[1], y) * (1 - smoothstep(MASK.bottom[0], MASK.bottom[1], y));
