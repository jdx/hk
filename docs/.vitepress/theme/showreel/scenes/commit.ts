// git commit runs hk (storyboard §6.3): the command everyone types slams
// onto the stage as big type, and Enter turns it into the first row of a
// terminal, where hk's pre-commit hook starts and fetches the staged files.
//
//   b0–1    the hk.pkl chip (config|commit) breathes, back to exactly 1 by b1
//   b1      `git` drops onto baseline 420: stretch, squash, 12 dust motes
//   b1.5    `commit` slides in from the right edge, smeared, and slams
//           against it as one body: each glyph squashes a 32nd of a beat
//           after the one before, the word closing up like an accordion
//           and springing back out, and the letters of `git` jostle
//   b2–4.25 `-m "feat: hoist the sails"` flips in word by word, a dotted
//           eighth apart (b2, 2.75, 3.5, 4.25, running on into Enter), the
//           cursor riding ahead of it, parked at each word's end as it opens
//   b5      Enter: the type squashes into the key from b4.84, the cursor
//           flashes, and from b5.05 the glyphs stream off in column
//           order, each flying to its cell
//           in row 0 of PANE_FULL, which grows out of the type's box; the
//           chip flashes and shrinks into the chrome bar. Row and pane are
//           at rest by b5.46, and the terminal draws the row itself on b5.5
//   b5.5    commit[0]: the header at 0/7 and the files step, spinner live
//   b6.5    commit[1]: ✔ files … (4 files): `git status` fades, the new
//           message sweeps in behind a soft edge, and the ✔ pops with a
//           green glow and ring, all done by b6.94
//   b7–12   exactly commit|stash (hk hides the cursor while it runs), under
//           the caption until its wipe ends on b11.75
//
// The big type holds the stage for two seconds (b1–5), since it is what
// reads on a phone, and the still pane after the ✔ is left only the
// caption's reading time.
//
// Every frame is a pure function of time. The spinner reads global time.
// From b6.94 nothing moves: the frame is commit|stash's to the pixel, and
// drawHandoff's itself from b7 to the bar line. At lt 0 it is
// config|commit's, the chip at rest.

import { BEAT, PALETTE, type Scene, type SceneEnv, sec, TERM } from "../bible";
import { mix, rgba } from "../color";
import { glow, makeCanvas, ring, roundedRect, shake } from "../fx";
import { bg, drawHandoff } from "../handoff";
import { drawChip, HKPKL_CHIP } from "../kit/card";
import { jolt, land, lerpRect, type Pt, type Rect } from "../kit/motion";
import { commit, PROMPT_COMMIT } from "../kit/screens";
import { advance, bodyOf, CHROME, drawTerm, drawTermLine, drawWindow, PANE_FULL, type Pane, RADIUS, termCharX, type TermLayout, termLayout, termLit } from "../kit/term";
import {
  clamp,
  cubicBezier,
  hash,
  inOutCubic,
  lerp,
  outCubic,
  progress,
  rng,
  smoothstep,
  swiftIn,
  swiftInOut,
  swiftOut,
  TAU,
} from "../math";
import { type Caption, entrance, font, MONO } from "../type";

const S = sec("commit");
const b = (n: number): number => n * BEAT;

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 5 + 5 = 10 words need 6 beats; held 6 from b5.5, as hk's rows print
  // (line 2 needs 3.5, holds 5.5). The wipe ends b11.75.
  { out: 11.5, lines: [{ in: 5.5, text: "After `hk install`, `git commit`" }, { in: 6, text: "runs hk before the commit." }] },
];

/**
 * The scene's hits, in section-local beats, for the score: `git` lands,
 * `commit` slams, the four flips, Enter, hk's first rows, the files ✔.
 */
export const COMMIT_BEATS = { git: 1, slam: 1.5, flips: [2, 2.75, 3.5, 4.25], enter: 5, run: 5.5, files: 6.5 } as const;

const T_GIT = b(COMMIT_BEATS.git);
const T_SLAM = b(COMMIT_BEATS.slam);
const T_ENTER = b(COMMIT_BEATS.enter);
const T_RUN = b(COMMIT_BEATS.run);
const T_FILES = b(COMMIT_BEATS.files);
/** From here to the bar line the frame is exactly commit|stash. */
const T_SETTLED = b(COMMIT_BEATS.files + 0.5);
/**
 * The pane has grown and the last glyph has landed in row 0 a few frames
 * before b5.5, so the row is at rest before the terminal draws it itself.
 */
const LANDED = T_RUN - 0.02;

// The command, verbatim from row 0 of the pane: `git commit` big, the rest
// under it.
const CMD = PROMPT_COMMIT.slice(2);
const BIG_TEXT = CMD.slice(0, CMD.indexOf(" -m"));
const SUB_TEXT = CMD.slice(BIG_TEXT.length + 1);
/** Row 0 column of the command's first character (after `$ `). */
const CMD_COL = 2;

/** `git commit`: bold mono 150 px warm, centred, x 510–1410, baseline 420. */
const BIG = { size: 150, adv: advance(150), x0: 960 - (BIG_TEXT.length * advance(150)) / 2, y: 420 };
/** `-m "feat: hoist the sails"`: bold mono 64 px, centred on baseline 540. */
const SUB = { size: 64, adv: advance(64), x0: 960 - (SUB_TEXT.length * advance(64)) / 2, y: 540 };
const WEIGHT = 700;
/** Where `commit` starts in BIG_TEXT. */
const COMMIT_AT = BIG_TEXT.indexOf(" ") + 1;

/** The four flips: `-m` warm, then the message, a string, in cyan. */
const FLIPS = (() => {
  let from = 0;
  return ["-m", '"feat:', "hoist", 'the sails"'].map((w, n) => {
    const i = SUB_TEXT.indexOf(w, from);
    from = i + w.length;
    return { from: i, to: i + w.length, at: b(COMMIT_BEATS.flips[n]), color: n === 0 ? PALETTE.warm : PALETTE.cyan };
  });
})();

interface Glyph {
  ch: string;
  /** In `git commit` (true) or the line under it. */
  big: boolean;
  /** Index in its line. */
  i: number;
  /** Its cell in row 0. */
  col: number;
  /** 0 `git`, 1 `commit`, 2.. the flips. */
  word: number;
  color: string;
  /** Order of departure at Enter: by column. */
  rank: number;
}

const GLYPHS: readonly Glyph[] = (() => {
  const out: Glyph[] = [];
  Array.from(BIG_TEXT).forEach((ch, i) => {
    if (ch !== " ") out.push({ ch, big: true, i, col: CMD_COL + i, word: i < COMMIT_AT ? 0 : 1, color: PALETTE.warm, rank: 0 });
  });
  Array.from(SUB_TEXT).forEach((ch, i) => {
    if (ch === " ") return;
    const w = FLIPS.findIndex((f) => i >= f.from && i < f.to);
    out.push({ ch, big: false, i, col: CMD_COL + BIG_TEXT.length + 1 + i, word: 2 + w, color: FLIPS[w].color, rank: 0 });
  });
  out.sort((a, c) => a.col - c.col).forEach((g, k) => (g.rank = k));
  return out;
})();

// Small motion helpers.

/** A knock's settle: A at `d` 0, oscillating at `f` Hz and exactly 0 from `dur`. */
function settle(d: number, amp: number, f: number, dur: number): number {
  if (d < 0 || d >= dur) return 0;
  return amp * Math.cos(TAU * f * d) * (1 - d / dur) ** 2;
}

/** A hit's flash: up over `attack` to 1 on `at`, then decaying with `tau`. */
function flashAt(t: number, at: number, tau: number, attack = 0.016): number {
  if (t < at - attack) return 0;
  if (t < at) return progress(at - attack, at, t);
  const v = Math.exp(-(t - at) / tau);
  return v < 0.004 ? 0 : v;
}

// The camera on the big type: a knock on each landing and a quick breath in
// just before Enter (the anticipation of the key). Exactly still otherwise:
// a slow drift would step the type a pixel at a time as the canvas snaps
// its glyphs. Never used after Enter (the flight starts where Enter left it).

const FOCUS: Pt = { x: 960, y: 520 };

interface Cam {
  z: number;
  dx: number;
  dy: number;
}

function knock(lt: number, at: number, amp: number, decay: number): [number, number] {
  const [x, y] = shake(lt, at, amp, decay);
  const k = 1 - smoothstep(at + 2 * decay, at + 3 * decay, lt);
  return [x * k, y * k];
}

/** The camera at `lt`; `depth` < 1 is further back (the chip), so it moves less. */
function camera(lt: number, depth = 1): Cam {
  const z = 1 + 0.03 * smoothstep(T_ENTER - b(0.4), T_ENTER, Math.min(lt, T_ENTER));
  const [ax, ay] = knock(lt, T_GIT, 9, 0.045);
  const [bx, by] = knock(lt, T_SLAM, 6, 0.04);
  return { z: 1 + (z - 1) * depth, dx: (ax + bx) * depth, dy: (ay + by) * depth };
}

const view = (c: Cam, x: number, y: number): Pt => ({ x: FOCUS.x + (x - FOCUS.x) * c.z + c.dx, y: FOCUS.y + (y - FOCUS.y) * c.z + c.dy });

// The big type, before Enter: where each glyph is and how it is deformed,
// in world px (the camera is applied by the caller).

interface Pose {
  /** The glyph's baseline centre. */
  x: number;
  y: number;
  size: number;
  sx: number;
  sy: number;
  rot: number;
  /** Horizontal shear about the baseline: the top's shift per px of height, + to the right. */
  skew?: number;
  alpha: number;
  /** 0..1 toward white: a flip catching the light. */
  tint: number;
}

/** `git` falls for this long before it lands on b1, from this far above. */
const FALL = 0.22;
const DROP = 700;
/** `commit`'s speed as it slides in, px/s, and its glyphs' stagger as they squash on impact. */
const SLIDE_V = 6400;
const PILE = BEAT / 32;

/** How far the slam shoves git's glyph `i` left, 0..1 (× SHOVE px): t first and hardest, the shove running left through the word a 32nd apart. */
const shove = (i: number, lt: number): number => jolt(lt, T_SLAM + (COMMIT_AT - 2 - i) * PILE, 0.42, 4.5);
const SHOVE = 20;

function gitPose(g: Glyph, lt: number): Pose | null {
  const x = BIG.x0 + (g.i + 0.5) * BIG.adv;
  if (lt < T_GIT - FALL) return null;
  if (lt < T_GIT) {
    // Accelerating, and stretched by the speed.
    const u = progress(T_GIT - FALL, T_GIT, lt);
    return {
      x,
      y: BIG.y - DROP * (1 - u * u),
      size: BIG.size,
      sx: 1 - 0.14 * u,
      sy: 1 + 0.36 * u,
      rot: (hash(g.i, 5) - 0.5) * 0.18 * (1 - u),
      alpha: 1,
      tint: 0,
    };
  }
  // Squashed flat on the landing, springing back through a stretch, then
  // knocked left by commit's slam.
  const sq = settle(lt - T_GIT, 0.34, 3.1, 0.5);
  const k = [0.45, 0.7, 1][g.i] ?? 1;
  const push = shove(g.i, lt);
  return { x: x - SHOVE * k * push, y: BIG.y, size: BIG.size, sx: 1 + 0.55 * sq, sy: 1 - sq, rot: -0.08 * k * push, alpha: 1, tint: 0 };
}

// commit slams as one body. Each glyph has a width (a fraction of its
// cell) and sits hard against the one before it, so the letters always
// touch as they would at rest: no gap opens and no two overlap, whatever
// the widths do. The c's left edge rides git's t; everything else follows
// from the widths.

/** While it slides each glyph is stretched this much along the move, the word leaning back this far. */
const SLIDE_W = 1.18;
const LEAN = 0.06;
/** On impact a glyph squashes to this width over IMPACT s, then springs back out, overshooting 3 % and exactly home BACK_DUR s later. */
const SQUASH_W = 0.8;
const IMPACT = 0.02;
const BACK_F = 2.6;
const BACK_DUR = 0.3;
/** The c's left edge at rest: its cell's. */
const CONTACT = BIG.x0 + COMMIT_AT * BIG.adv;

/** A spring back from 1 to 0: level at x 0, 3 % past 0 at 0.17 s and exactly 0, still, from BACK_DUR. */
function springBack(x: number): number {
  if (x >= BACK_DUR) return 0;
  const w = TAU * BACK_F;
  const a = 2 / (BACK_DUR * w);
  return (1 - x / BACK_DUR) ** 2 * (Math.cos(w * x) + a * Math.sin(w * x));
}

/** commit's glyph `k`'s width at `lt`: stretched, squashed on impact (fast at first, so the glyphs behind keep their speed), then back. */
function commitWidth(k: number, lt: number): number {
  const d = lt - (T_SLAM + k * PILE);
  if (d <= 0) return SLIDE_W;
  if (d < IMPACT) return lerp(SLIDE_W, SQUASH_W, outCubic(d / IMPACT));
  return 1 - (1 - SQUASH_W) * springBack(d - IMPACT);
}

/** The word's lean: back into the slide, whipping forward on impact and settling, exactly 0 from T_SLAM + 0.4. One for the whole word, so tops never collide. */
function commitLean(lt: number): number {
  if (lt < T_SLAM) return LEAN;
  const d = lt - T_SLAM;
  const dur = 0.4;
  if (d >= dur) return 0;
  const w = TAU * 4;
  return (1 - d / dur) ** 2 * (LEAN * Math.cos(w * d) - 0.12 * Math.sin(w * d));
}

function commitPose(g: Glyph, lt: number): Pose | null {
  const k = g.i - COMMIT_AT;
  let left = lt < T_SLAM ? CONTACT + SLIDE_V * (T_SLAM - lt) : CONTACT - SHOVE * shove(COMMIT_AT - 2, lt);
  for (let j = 0; j < k; j++) left += BIG.adv * commitWidth(j, lt);
  if (left > 2000) return null;
  const w = commitWidth(k, lt);
  return {
    x: left + (BIG.adv * w) / 2,
    y: BIG.y,
    size: BIG.size,
    sx: w,
    // Taller as it is squashed, lower as it is stretched.
    sy: 1 - 0.33 * (w - 1),
    rot: 0,
    skew: commitLean(lt),
    alpha: 1,
    tint: 0,
  };
}

/** A word's card flip, 0..1 with a little overshoot, exactly 1 from at + 0.14. */
const flipK = (lt: number, at: number): number => land(lt, at - 0.03, 0.17, 0.12);

function flipPose(g: Glyph, lt: number): Pose | null {
  const f = FLIPS[g.word - 2];
  const sy = flipK(lt, f.at);
  if (sy <= 0) return null;
  // Flipping about the x-height's middle.
  const h = 0.36 * SUB.size;
  return {
    x: SUB.x0 + (g.i + 0.5) * SUB.adv,
    y: SUB.y - (1 - sy) * h,
    size: SUB.size,
    sx: 1,
    sy,
    rot: 0,
    alpha: clamp(sy * 3),
    tint: 0.55 * (1 - clamp(sy)),
  };
}

/** `git commit` bobbing as each word flips in under it: exactly 0 from before Enter. */
function bob(lt: number): number {
  let dy = 0;
  for (const f of FLIPS) dy -= 4 * jolt(lt, f.at, 0.2, 6);
  return dy;
}

function restPose(g: Glyph, lt: number): Pose | null {
  if (!g.big) return flipPose(g, lt);
  const p = g.word === 0 ? gitPose(g, lt) : commitPose(g, lt);
  return p && { ...p, y: p.y + bob(lt) };
}

/** A pose seen through the camera. */
function seen(p: Pose, c: Cam): Pose {
  const q = view(c, p.x, p.y);
  return { ...p, x: q.x, y: q.y, size: p.size * c.z };
}

// The pane, growing out of the type's box over b5–5.5.

/** The type's box at rest (world px): `git commit`'s ascenders to the message's descenders, the cursor included, padded. */
const TYPE_BOX: Rect = { x: SUB.x0 - 44, y: BIG.y - 0.76 * BIG.size - 36, w: 0, h: 0 };
TYPE_BOX.w = SUB.x0 + (SUB_TEXT.length + 1) * SUB.adv + 44 - TYPE_BOX.x;
TYPE_BOX.h = SUB.y + 0.24 * SUB.size + 36 - TYPE_BOX.y;

/** The pane's growth, 0..1, with a small overshoot, exactly 1 (and still) from LANDED. */
const growK = (lt: number): number => land(lt, T_ENTER, LANDED - T_ENTER, 0.05);

function startBox(): Rect {
  const c = camera(T_ENTER);
  const a = view(c, TYPE_BOX.x, TYPE_BOX.y);
  const z = view(c, TYPE_BOX.x + TYPE_BOX.w, TYPE_BOX.y + TYPE_BOX.h);
  return { x: a.x, y: a.y, w: z.x - a.x, h: z.y - a.y };
}
const START_BOX = startBox();

/** PANE_FULL as it stands at `lt`: its grid moves with its top-left corner. */
function paneAt(lt: number): Pane {
  const r = lerpRect(START_BOX, PANE_FULL, growK(lt));
  return {
    ...PANE_FULL,
    ...r,
    x0: r.x + (PANE_FULL.x0 - PANE_FULL.x),
    baseline0: r.y + (PANE_FULL.baseline0 - PANE_FULL.y),
  };
}

/** How far the window has come up. */
const windowK = (lt: number): number => smoothstep(T_ENTER, T_ENTER + 0.08, lt);

/**
 * Where the pane at `lt` draws row 0's character `col`: its baseline centre,
 * as termCharX gives it, so the glyphs land exactly where the row the
 * terminal then draws itself has them.
 */
function cellAt(ctx: CanvasRenderingContext2D, col: number, lt: number): Pt {
  const L = termLayout(paneAt(lt), 3);
  const c = termCharX(ctx, PROMPT_COMMIT, col, L.col(0), PANE_FULL.size);
  return { x: c.x + c.w / 2, y: L.baseline(0) };
}

// The flight into row 0.

/**
 * The first glyph leaves this long after Enter (b5.05), the type held
 * squashed till then; they leave in column order, this far apart, so the
 * row visibly streams off the big type.
 */
const DEPART = 0.025;
const STAGGER = 0.0035;
/**
 * The key's press: the type squashes into it over the PRESS s before the
 * beat (anticipation, on the camera's push-in), to 1 + PRESS_SX wide and
 * 1 − PRESS_SY tall at Enter, and is let go glyph by glyph into the flight.
 */
const PRESS = 0.08;
const PRESS_SX = 0.08;
const PRESS_SY = 0.2;
const pressK = (lt: number): number => smoothstep(T_ENTER - PRESS, T_ENTER, lt);
/** Each flight is as long as lets the last glyph land on LANDED. */
const FLY = LANDED - T_ENTER - DEPART - (GLYPHS.length - 1) * STAGGER;
/** Eased in and out, the travel spread over the flight rather than spent in its first frames. */
const FLY_EASE = cubicBezier(0.45, 0, 0.2, 1);

interface Flight extends Pose {
  color: string;
  /** 1 bold (the big type's weight), 0 regular (the terminal's). */
  bold: number;
  /** 0..1 how far along. */
  p: number;
}

function flight(ctx: CanvasRenderingContext2D, g: Glyph, lt: number): Flight {
  const src = seen(restPose(g, T_ENTER)!, camera(T_ENTER));
  const td = T_ENTER + DEPART + g.rank * STAGGER;
  const ta = td + FLY;
  // Squashed by the key (drawType has it building up to the beat), then let go into the flight.
  const press = pressK(lt) * (1 - smoothstep(td, td + 0.4 * FLY, lt));
  const u = progress(td, ta, lt);
  const p = u >= 1 ? 1 : FLY_EASE(u);
  const dst = cellAt(ctx, g.col, lt);
  // A hop: up over the straight line, more for glyphs going further.
  const dist = Math.hypot(dst.x - src.x, dst.y - src.y);
  const lift = (0.09 + 0.03 * hash(g.rank, 13)) * dist;
  const cx = (src.x + dst.x) / 2;
  const cy = (src.y + dst.y) / 2 - lift;
  const v = 1 - p;
  const x = v * v * src.x + 2 * v * p * cx + p * p * dst.x;
  const y = v * v * src.y + 2 * v * p * cy + p * p * dst.y;
  // Exactly the terminal's size once landed, so it rasterizes as drawTerm's row does.
  const size = p >= 1 ? PANE_FULL.size : Math.exp(lerp(Math.log(src.size), Math.log(PANE_FULL.size), p));
  return {
    x,
    y,
    size,
    sx: 1 + PRESS_SX * press,
    sy: 1 - PRESS_SY * press,
    rot: 0,
    alpha: 1,
    tint: 0,
    color: mix(g.color, TERM.text, smoothstep(0.25, 0.85, p)),
    bold: 1 - smoothstep(0.45, 0.95, p),
    p: u,
  };
}

// Drawing.

/** One glyph centred on (x, y)'s baseline, deformed about that point. */
function drawGlyph(
  ctx: CanvasRenderingContext2D,
  ch: string,
  x: number,
  y: number,
  size: number,
  weight: number,
  color: string,
  o: { sx?: number; sy?: number; rot?: number; skew?: number; alpha?: number } = {},
): void {
  const a = o.alpha ?? 1;
  if (ch === " " || a <= 0.003 || size < 0.5) return;
  ctx.save();
  ctx.translate(x, y);
  if (o.rot) ctx.rotate(o.rot);
  if (o.skew) ctx.transform(1, 0, -o.skew, 1, 0, 0);
  ctx.scale(o.sx ?? 1, o.sy ?? 1);
  ctx.globalAlpha *= a;
  ctx.font = font(size, weight, MONO);
  ctx.textAlign = "center";
  ctx.textBaseline = "alphabetic";
  ctx.fillStyle = color;
  ctx.fillText(ch, 0, 0);
  ctx.restore();
}

function drawPose(ctx: CanvasRenderingContext2D, g: Glyph, p: Pose, alpha = 1): void {
  const color = p.tint > 0 ? mix(g.color, "#ffffff", p.tint) : g.color;
  drawGlyph(ctx, g.ch, p.x, p.y, p.size, WEIGHT, color, { sx: p.sx, sy: p.sy, rot: p.rot, skew: p.skew, alpha: p.alpha * alpha });
}

/** commit's smear runs on through the impact while the glyphs behind the c are still closing up. */
const SMEAR_END = T_SLAM + 5 * PILE + IMPACT;

/** The big type before Enter, with commit's smear while it slides and squashing into the key's press. */
function drawType(ctx: CanvasRenderingContext2D, lt: number): void {
  const c = camera(lt);
  const press = pressK(lt);
  for (const g of GLYPHS) {
    const p = restPose(g, lt);
    if (!p) continue;
    if (g.word === 1 && lt < SMEAR_END) {
      // Motion smear: fainter copies along where it was a shutter ago,
      // close enough (under 5 px at full speed) to merge into one streak,
      // fading along it and out as the word comes to rest.
      const N = 32;
      const k = 1 - smoothstep(T_SLAM, SMEAR_END, lt);
      for (let m = N; m >= 1; m--) {
        const q = restPose(g, lt - (m / N) * (1 / 45));
        if (q && Math.abs(q.x - p.x) > 1.5) drawPose(ctx, g, { ...seen(q, c), sx: p.sx, sy: p.sy, rot: p.rot, skew: p.skew }, 0.06 * k * (1 - m / (N + 1)) ** 1.5);
      }
    }
    if (g.word === 0 && lt < T_GIT) {
      // A short vertical blur on the drop, as the flight's: many close
      // copies over the shutter (4 px apart at the landing), fading along it.
      const N = 24;
      for (let m = N; m >= 1; m--) {
        const q = restPose(g, lt - (m / N) * (1 / 60));
        if (q && Math.abs(q.y - p.y) >= 2) drawPose(ctx, g, seen(q, c), 0.09 * (1 - m / (N + 1)) ** 1.5);
      }
    }
    const s = seen(p, c);
    drawPose(ctx, g, press > 0 ? { ...s, sx: s.sx * (1 + PRESS_SX * press), sy: s.sy * (1 - PRESS_SY * press) } : s);
  }
}

/** Each glyph on its way to row 0 (b5–5.5), and the prompt's `$` coming up in col 0. */
function drawFlight(ctx: CanvasRenderingContext2D, lt: number): void {
  for (const g of GLYPHS) {
    const f = flight(ctx, g, lt);
    if (f.p > 0 && f.p < 1) {
      // A motion blur: the glyph at many close points of a short shutter,
      // fading along it, so the copies merge into one streak.
      const N = 14;
      for (let m = N; m >= 1; m--) {
        const q = flight(ctx, g, lt - (m / N) * (1 / 150));
        if (Math.hypot(q.x - f.x, q.y - f.y) < 2) continue;
        drawFlightGlyph(ctx, g, { ...q, color: f.color, bold: f.bold }, 0.15 * (1 - m / (N + 1)) ** 1.5);
      }
    }
    drawFlightGlyph(ctx, g, f, 1);
  }
  // The prompt's `$`: up as the first glyphs arrive.
  const k = land(lt, T_ENTER + 0.13, 0.1, 0.2);
  if (k > 0) {
    const at = cellAt(ctx, 0, lt);
    drawGlyph(ctx, "$", at.x, at.y, PANE_FULL.size, 400, TERM.prompt, { sx: k, sy: k, alpha: clamp(k * 2) });
  }
}

function drawFlightGlyph(ctx: CanvasRenderingContext2D, g: Glyph, f: Flight, alpha: number): void {
  const o = { sx: f.sx, sy: f.sy, rot: f.rot };
  // The big type's bold hands over to the terminal's regular as it lands.
  if (f.bold > 0) drawGlyph(ctx, g.ch, f.x, f.y, f.size, WEIGHT, f.color, { ...o, alpha: alpha * f.bold });
  if (f.bold < 1) drawGlyph(ctx, g.ch, f.x, f.y, f.size, 400, f.color, { ...o, alpha: alpha * (1 - f.bold) });
}

// The cursor: a rosewater block after the last word typed, from `-m` to
// Enter, when it flashes and goes (hk hides it while it runs). It rides
// ahead of the typing: it glides to the end of each word before the word
// flips open behind it, so it never crosses the letters.

const CURSOR_H = 1.12 * SUB.size;
/** Each glide, before its word's flip: from at − GLIDE_FROM to at − GLIDE_TO (the flip opens from at − 0.03). */
const GLIDE_FROM = 0.07;
const GLIDE_TO = 0.02;

function cursorAt(lt: number): { x: number; y: number; w: number; h: number; alpha: number; flash: number } | null {
  const first = FLIPS[0].at - 0.03;
  if (lt < first || lt >= T_ENTER + 0.07) return null;
  // The words whose glide has begun, `-m` (which the cursor opens with) included.
  let n = 1;
  while (n < FLIPS.length && lt >= FLIPS[n].at - GLIDE_FROM) n++;
  const colX = (col: number) => SUB.x0 + col * SUB.adv;
  const at = FLIPS[n - 1].at;
  const glide = swiftOut(progress(at - GLIDE_FROM, at - GLIDE_TO, lt));
  const x = n === 1 ? colX(FLIPS[0].to) : lerp(colX(FLIPS[n - 2].to), colX(FLIPS[n - 1].to), glide);
  const sy = n === 1 ? flipK(lt, FLIPS[0].at) : 1;
  const flash = flashAt(lt, T_ENTER, 0.07);
  const gone = swiftIn(progress(T_ENTER, T_ENTER + 0.07, lt));
  const h = CURSOR_H * sy * (1 - gone);
  const mid = SUB.y - 0.8 * SUB.size + CURSOR_H / 2 - 0.04 * SUB.size;
  return { x, y: mid - h / 2, w: SUB.adv, h, alpha: 0.9, flash };
}

function drawCursor(ctx: CanvasRenderingContext2D, lt: number): void {
  const k = cursorAt(lt);
  if (!k || k.h <= 0.5) return;
  const c = camera(lt);
  const a = view(c, k.x, k.y);
  const z = view(c, k.x + k.w, k.y + k.h);
  ctx.save();
  ctx.globalAlpha *= k.alpha;
  ctx.fillStyle = mix(TERM.cursor, "#ffffff", 0.7 * k.flash);
  ctx.fillRect(a.x, a.y, z.x - a.x, z.y - a.y);
  ctx.restore();
}

// Particles and light.

/** Twelve seeded dust motes kicked out from under `git` as it lands. */
const MOTES = (() => {
  const r = rng(0xc0331);
  const mid = BIG.x0 + 1.5 * BIG.adv;
  return Array.from({ length: 12 }, () => {
    const x = BIG.x0 + 0.1 * BIG.adv + r() * 2.8 * BIG.adv;
    const side = (x - mid) / (1.5 * BIG.adv);
    return {
      x,
      vx: side * (160 + r() * 300) + (r() - 0.5) * 80,
      vy: -(90 + r() * 260),
      size: 3 + r() * 4,
      life: 0.42 + r() * 0.38,
      delay: r() * 0.03,
      warm: r(),
    };
  });
})();

function drawDust(ctx: CanvasRenderingContext2D, lt: number): void {
  const d0 = lt - T_GIT;
  const pre = progress(T_GIT - 0.12, T_GIT, lt);
  if (pre <= 0 || d0 > 1) return;
  const c = camera(lt);
  ctx.save();
  // The floor's contact line: lit as the word comes down on it, then
  // spreading out from under it.
  const fl = progress(0, 0.34, d0);
  if (fl < 1) {
    const mid = BIG.x0 + 1.5 * BIG.adv;
    const half = d0 < 0 ? 130 : 150 + 260 * outCubic(fl);
    const alpha = d0 < 0 ? 0.3 * pre * pre : 0.55 * (1 - fl) ** 1.5;
    const a = view(c, mid - half, BIG.y + 6);
    const z = view(c, mid + half, BIG.y + 6);
    const g = ctx.createLinearGradient(a.x, 0, z.x, 0);
    const col = PALETTE.warm;
    g.addColorStop(0, rgba(col, 0));
    g.addColorStop(0.5, rgba(col, alpha));
    g.addColorStop(1, rgba(col, 0));
    ctx.fillStyle = g;
    ctx.fillRect(a.x, a.y - 1.5, z.x - a.x, 3);
  }
  for (const m of MOTES) {
    const d = d0 - m.delay;
    if (d <= 0 || d >= m.life) continue;
    const x = m.x + m.vx * d;
    const y = BIG.y + 4 + m.vy * d + 0.5 * 1500 * d * d;
    if (y > BIG.y + 10) continue;
    const a = 0.75 * (1 - smoothstep(0.35 * m.life, m.life, d));
    const p = view(c, x, y);
    ctx.fillStyle = rgba(mix(PALETTE.warm, PALETTE.paperDim, m.warm), a);
    ctx.beginPath();
    ctx.arc(p.x, p.y, m.size * (1 - 0.4 * (d / m.life)), 0, TAU);
    ctx.fill();
  }
  ctx.restore();
}

/** The caption's first word starts to rise (b4.875). */
const T_CAPTION = entrance(CAPTIONS[0].lines[0].text, b(CAPTIONS[0].lines[0].in));

/** A warm light under the type while it is up, kicked by each landing. */
function drawTypeLight(ctx: CanvasRenderingContext2D, lt: number): void {
  if (lt < T_GIT - 0.05 || lt > T_ENTER) return;
  const c = camera(lt);
  // Out of the captions' band before the caption starts to rise.
  const up = smoothstep(T_GIT - 0.05, T_GIT + 0.12, lt) * (1 - smoothstep(T_CAPTION - b(0.475), T_CAPTION - b(0.075), lt));
  const base = view(c, 960, 430);
  glow(ctx, base.x, base.y, 760 * c.z, PALETTE.warmDeep, 0.07 * up);
  const hitGit = flashAt(lt, T_GIT, 0.1);
  if (hitGit > 0) {
    const p = view(c, BIG.x0 + 1.5 * BIG.adv, BIG.y - 20);
    glow(ctx, p.x, p.y, 340, PALETTE.warm, 0.22 * hitGit);
  }
  const hitSlam = flashAt(lt, T_SLAM, 0.08);
  if (hitSlam > 0) {
    const p = view(c, BIG.x0 + (COMMIT_AT - 0.5) * BIG.adv, BIG.y - 0.3 * BIG.size);
    glow(ctx, p.x, p.y, 240, PALETTE.warm, 0.24 * hitSlam);
  }
}

// The chip: breathing, riding the camera from further back, and at Enter
// flashing and shrinking into the chrome bar.

const CHIP_C: Pt = { x: HKPKL_CHIP.x + HKPKL_CHIP.w / 2, y: HKPKL_CHIP.y + HKPKL_CHIP.h / 2 };
const CHIP_DEPTH = 0.5;

/**
 * The chip's label, drawn once by drawChip onto an opaque canvas in the
 * chip's fill (so its text is antialiased as on the reel's own opaque
 * canvas), cut to LABEL. While the chip moves and scales slowly (the
 * breath, the knocks, the push-in, Enter's flash) its label is drawn from
 * it, since a bitmap scales and moves continuously where drawChip's label,
 * rasterized afresh each frame, steps its glyphs a whole pixel at a time.
 * Placed at rest it lands on whole pixels, so it is drawChip's label to
 * the pixel there.
 */
const LABEL: Rect = { x: 870, y: 126, w: 180, h: 50 };
/** Below this scale (from about b5.18, shrinking 7 % a frame) the label is drawn afresh: a bitmap scaled down further would alias. */
const LABEL_MIN_SCALE = 0.8;
let labelSprite: HTMLCanvasElement | null = null;

function spriteOfLabel(): HTMLCanvasElement | null {
  if (labelSprite) return labelSprite;
  // Not before the label's face is in, or the cache would keep a fallback's.
  if (typeof document === "undefined" || !document.fonts?.check(font(36, 400, MONO))) return null;
  const c = makeCanvas(LABEL.w, LABEL.h);
  const g = c.getContext("2d", { alpha: false });
  if (!g) return null;
  g.fillStyle = PALETTE.surface;
  g.fillRect(0, 0, LABEL.w, LABEL.h);
  g.translate(-LABEL.x, -LABEL.y);
  drawChip(g);
  labelSprite = c;
  return c;
}

/** The label's sprite scaled by `scale` about the chip's `centre`. */
function drawLabelSprite(ctx: CanvasRenderingContext2D, sprite: HTMLCanvasElement, centre: Pt, scale: number): void {
  ctx.save();
  // Bilinear: it interpolates, so at scale 1 on whole pixels it is an exact copy.
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "low";
  ctx.drawImage(sprite, centre.x + (LABEL.x - CHIP_C.x) * scale, centre.y + (LABEL.y - CHIP_C.y) * scale, LABEL.w * scale, LABEL.h * scale);
  ctx.restore();
}

function drawTheChip(ctx: CanvasRenderingContext2D, lt: number): void {
  const flash = flashAt(lt, T_ENTER, 0.09);
  let centre: Pt;
  let scale: number;
  let alpha = 1;
  if (lt < T_ENTER) {
    const c = camera(lt, CHIP_DEPTH);
    centre = view(c, CHIP_C.x, CHIP_C.y);
    // One breath, ±2 %, still at both ends; exactly 1 from b1.
    const p = progress(0, b(1), lt);
    scale = c.z * (1 + 0.026 * Math.sin(TAU * p) * Math.sin(Math.PI * p));
  } else {
    const c = camera(T_ENTER, CHIP_DEPTH);
    const from = view(c, CHIP_C.x, CHIP_C.y);
    const pane = paneAt(lt);
    const to = { x: pane.x + pane.w / 2, y: pane.y + CHROME / 2 };
    const m = swiftInOut(progress(T_ENTER, T_RUN - 0.04, lt));
    centre = { x: lerp(from.x, to.x, m), y: lerp(from.y, to.y, m) };
    scale = c.z * (1 - inOutCubic(progress(T_ENTER + 0.02, T_RUN - 0.04, lt)));
    alpha = 1 - smoothstep(T_RUN - 0.12, T_RUN - 0.04, lt);
  }
  if (alpha <= 0 || scale <= 0.01) return;
  const rect = { x: centre.x - HKPKL_CHIP.w / 2, y: centre.y - HKPKL_CHIP.h / 2, w: HKPKL_CHIP.w, h: HKPKL_CHIP.h };
  if (flash > 0) glow(ctx, centre.x, centre.y, 200 * scale, PALETTE.cyan, 0.5 * flash * alpha);
  // The sprite is opaque, so only while the chip is.
  const sprite = alpha === 1 && scale >= LABEL_MIN_SCALE ? spriteOfLabel() : null;
  drawChip(ctx, { rect, scale, alpha, stroke: flash > 0 ? mix(PALETTE.cyan, "#ffffff", 0.7 * flash) : undefined, textAlpha: sprite ? 0 : 1 });
  if (sprite) drawLabelSprite(ctx, sprite, centre, scale);
}

/**
 * The chip's light running out along the chrome bar as it is taken in:
 * rising from b5.3, while the chip is still a speck, to its peak as the
 * chip's scale reaches 0 (T_RUN − 0.04), and gone by b6.06.
 */
function drawChromeGlint(ctx: CanvasRenderingContext2D, lt: number): void {
  const t0 = T_RUN - 0.1;
  const p = progress(t0, t0 + 0.38, lt);
  if (p <= 0 || p >= 1) return;
  const pane = paneAt(lt);
  const cx = pane.x + pane.w / 2;
  const y = pane.y + CHROME - 1;
  const half = (pane.w / 2) * outCubic(p);
  const a = smoothstep(t0, T_RUN - 0.04, lt) * (1 - p) ** 1.6;
  ctx.save();
  const g = ctx.createLinearGradient(cx - half, 0, cx + half, 0);
  g.addColorStop(0, rgba(PALETTE.cyan, 0));
  g.addColorStop(0.5, rgba(PALETTE.cyanBright, 0.8 * a));
  g.addColorStop(1, rgba(PALETTE.cyan, 0));
  ctx.fillStyle = g;
  ctx.fillRect(cx - half, y - 1, 2 * half, 2);
  ctx.restore();
  glow(ctx, cx, pane.y + CHROME / 2, 120, PALETTE.cyan, 0.45 * a);
}

/** Enter's light, at the cursor. */
function drawEnterFlash(ctx: CanvasRenderingContext2D, lt: number): void {
  const f = flashAt(lt, T_ENTER, 0.05);
  const r = progress(T_ENTER, T_ENTER + 0.14, lt);
  if (f <= 0 && (r <= 0 || r >= 1)) return;
  const c = camera(T_ENTER);
  const x = SUB.x0 + (FLIPS[FLIPS.length - 1].to + 0.5) * SUB.adv;
  const p = view(c, x, SUB.y - 0.3 * SUB.size);
  // Gone well before its tail can band on the pane that opens under it.
  glow(ctx, p.x, p.y, 220, TERM.cursor, 0.55 * f * (1 - progress(T_ENTER + 0.05, T_ENTER + 0.12, lt)));
  ring(ctx, p.x, p.y, 90, r, PALETTE.cyanBright, 3);
}

// hk's rows.

/** Draw inside the pane's body, clipped as drawTerm clips its lines, so a row at rest rasterizes exactly as the handoff's does. */
function inPane(ctx: CanvasRenderingContext2D, draw: () => void): void {
  const body = bodyOf(PANE_FULL);
  ctx.save();
  roundedRect(ctx, PANE_FULL.x, PANE_FULL.y, PANE_FULL.w, PANE_FULL.h, RADIUS);
  ctx.clip();
  ctx.beginPath();
  ctx.rect(body.x, body.y, body.w, body.h);
  ctx.clip();
  draw();
  ctx.restore();
}

/** A row printed on, left to right, from `at` over `dur`. */
function printRow(ctx: CanvasRenderingContext2D, L: TermLayout, text: string, row: number, lt: number, at: number, dur: number, t: number): void {
  const p = outCubic(progress(at, at + dur, lt));
  if (p <= 0) return;
  const y = L.baseline(row);
  ctx.save();
  if (p < 1) {
    const edge = lerp(L.col(0) - 4, L.col(Array.from(text).length) + 4, p);
    ctx.beginPath();
    ctx.rect(0, y - 40, edge, 60);
    ctx.clip();
  }
  drawTermLine(ctx, text, L.col(0), y, PANE_FULL.size, { t });
  ctx.restore();
}

/** Row 2's redraw starts here (the old tail starting to fade), and everything on it is done by FILES_DONE (b6.94): from there the frame is commit|stash's to the pixel, before the switch to drawHandoff on b7. */
const FILES_FROM = T_FILES - 0.07;
const FILES_DONE = T_FILES + 0.22;
/** The ✔'s ring's last radius, px: inside its cell pair and under row 1's baseline. */
const RING_R = 28;

/**
 * Row 2 redrawn as the files step finishes: the old message's tail (`git
 * status`) fades out, the new one (`staged files (4 files)`) sweeps in
 * behind a soft edge, and the ✔ pops in where the spinner was.
 */
function drawFilesRow(ctx: CanvasRenderingContext2D, L: TermLayout, lt: number, t: number): void {
  const before = commit[0][1];
  const after = commit[1][1];
  const y = L.baseline(2);
  // The shared head, after the state glyph's column: ` files - Fetching `.
  let same = 1;
  while (same < after.length && before[same] === after[same]) same++;
  const tailX = L.col(same);
  const clip = (x0: number, x1: number, draw: () => void) => {
    ctx.save();
    ctx.beginPath();
    ctx.rect(x0, y - 40, x1 - x0, 60);
    ctx.clip();
    draw();
    ctx.restore();
  };
  inPane(ctx, () => {
    // The head, as it was and is.
    clip(0, tailX, () => drawTermLine(ctx, after.slice(1), L.col(1), y, PANE_FULL.size, { t }));
    // The old tail going.
    const out = 1 - smoothstep(FILES_FROM, T_FILES - 0.025, lt);
    if (out > 0) {
      ctx.save();
      ctx.globalAlpha *= out;
      clip(tailX, 1920, () => drawTermLine(ctx, before.slice(1), L.col(1), y, PANE_FULL.size, { t }));
      ctx.restore();
    }
    // The new tail coming, its leading edge feathered over FEATHER px.
    const FEATHER = 80;
    const e = outCubic(progress(T_FILES - 0.025, T_FILES + 0.1, lt));
    if (e > 0) {
      const edge = lerp(tailX, L.col(Array.from(after).length) + FEATHER, e);
      clip(tailX, 1920, () => drawTermLine(ctx, after.slice(1), L.col(1), y, PANE_FULL.size, { t, fade: e < 1 ? [edge - FEATHER, edge] : null }));
    }
  });
  // The glyph: the spinner shrinks away and the ✔ pops in over it.
  const cell = L.cell(2, 0);
  const cx = cell.x + cell.w / 2;
  const cy = y - 0.33 * PANE_FULL.size;
  const scaled = (text: string, k: number) => {
    if (k <= 0.01) return;
    ctx.save();
    // At rest, no transform at all, so it rasterizes exactly as drawTerm's row does.
    if (k !== 1) {
      ctx.translate(cx, cy);
      ctx.scale(k, k);
      ctx.translate(-cx, -cy);
    }
    drawTermLine(ctx, text, L.col(0), y, PANE_FULL.size, { t });
    ctx.restore();
  };
  // The ✔ is well on its way on the beat itself.
  inPane(ctx, () => scaled(`${before[0]} `, 1 - smoothstep(T_FILES - 0.055, T_FILES - 0.02, lt)));
  const pop = land(lt, T_FILES - 0.03, 0.22, 0.2);
  // Its light: up with the pop to its peak on the beat, gone by FILES_DONE.
  const lit = lt < T_FILES ? smoothstep(T_FILES - 0.03, T_FILES, lt) : 1 - progress(T_FILES, FILES_DONE, lt);
  if (lit > 0) glow(ctx, cx, cy, 54, TERM.green, 0.45 * lit * lit);
  inPane(ctx, () => scaled(`${after[0]} `, pop));
  // A small ring pulsing out from round the ✔ as it lands, gone by
  // FILES_DONE. It stops short of row 1's `hk` and of the `f` of `files`.
  const rp = progress(T_FILES + 0.03, FILES_DONE, lt);
  if (rp > 0 && rp < 1) {
    ctx.save();
    ctx.strokeStyle = rgba(TERM.green, 0.85 * (1 - rp) ** 2);
    ctx.lineWidth = 0.5 + 2.5 * (1 - rp);
    ctx.beginPath();
    ctx.arc(cx, cy, lerp(14, RING_R, outCubic(rp)), 0, TAU);
    ctx.stroke();
    ctx.restore();
  }
}

function drawRun(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  drawTerm(ctx, PANE_FULL, [PROMPT_COMMIT], { t: env.t });
  const L = termLayout(PANE_FULL, 3);
  inPane(ctx, () => {
    printRow(ctx, L, commit[0][0], 1, lt, T_RUN, 0.07, env.t);
    if (lt < FILES_FROM) printRow(ctx, L, commit[0][1], 2, lt, T_RUN + 0.035, 0.06, env.t);
  });
  if (lt >= FILES_FROM) drawFilesRow(ctx, L, lt, env.t);
}

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    if (lt >= T_SETTLED) {
      drawHandoff(ctx, "commit|stash", env);
      return;
    }
    if (lt <= 0) {
      drawHandoff(ctx, "config|commit", env);
      return;
    }
    ctx.save();
    bg(ctx, env);
    drawTypeLight(ctx, lt);
    if (lt < T_RUN) {
      // The window grows out of the type's box.
      const w = windowK(lt);
      if (w > 0) {
        ctx.save();
        ctx.globalAlpha *= w;
        drawWindow(ctx, paneAt(lt), true);
        ctx.restore();
      }
    } else {
      drawRun(ctx, lt, env);
    }
    drawTheChip(ctx, lt);
    drawChromeGlint(ctx, lt);
    drawDust(ctx, lt);
    drawCursor(ctx, lt);
    if (lt < T_ENTER) drawType(ctx, lt);
    else if (lt < T_RUN) drawFlight(ctx, lt);
    drawEnterFlash(ctx, lt);
    ctx.restore();
  },
  lit(lt) {
    if (lt < T_ENTER) return null;
    if (lt < T_RUN) return termLit(paneAt(lt), windowK(lt));
    return termLit(PANE_FULL);
  },
  captions: () => CAPTIONS,
};
