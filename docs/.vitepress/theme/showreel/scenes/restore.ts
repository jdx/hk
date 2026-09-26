// Section 6, "Fixes staged, edits back" (storyboard §6.6), 30–36 s. Three
// beats of story:
//
// 1. Staged. The finished run from `lanes` rewinds: a warm edge sweeps the
//    bars and their tracks right to left into the file labels, like a tape
//    measure, and each file takes a cyan `✓ staged` stamp in a 32nd
//    cascade. README.md's fixed content is the last commit's, so its row
//    dims and its stamp turns to `= HEAD`: that is why the commit lists
//    three files. Its row is the last of the lanes to go, stepping up clear
//    of the split and holding a beat.
// 2. Edits back. The split: src/main.py as staged (the fixed file) on the
//    left, its worktree copy on the right. The fixed lines slide across the
//    divider as a copy (the fixer's result reaches the worktree too), then
//    the stash tray opens and the unstaged TODO line rises out of it and
//    zips into the worktree copy glyph by glyph, its teeth meshing: hk's
//    three-way merge of base, fixer output and worktree.
// 3. Committed. The split closes onto the staged side, which goes into the
//    terminal as the commit: `[main ada2ca4] feat: hoist the sails` prints,
//    lifts off as a cyan dot and shoots along `main`, landing on the bell.
//    The commit is hk's cyan throughout, never logo cyan (storyboard §3).
//
// Starts on lanes|restore (the finished Gantt and the closed tray) and is
// restore|catch (the main line) from b8, its head's glow breathing once, and
// exactly from b11. Nothing here reads the facts. Every terminal line is
// commit.frames.txt's (kit/screens.ts). The TODO strip is stash's own
// (scenes/stash-peel.ts).

import { BEAT, LANE, PALETTE, type Scene, type SceneEnv, sec } from "../bible";
import { mix, rgba } from "../color";
import { glow, ring, roundedRect } from "../fx";
import { bg, drawHandoff } from "../handoff";
import { type CardRect, cardLayout, drawCard, drawMono, MAIN_PY_FIXED, MAIN_PY_TODO, monoWidth } from "../kit/card";
import { barSpan, drawBar, drawDone, drawLanes, LANE_FILES, LANES, SCHEDULE } from "../kit/lanes";
import { drawCommitDot, drawMain, MAIN } from "../kit/mainline";
import { bump, jolt, land, type Pt } from "../kit/motion";
import { commit } from "../kit/screens";
import { drawTerm, type Pane, STRIP, termLayout, termLit } from "../kit/term";
import { drawTray, TRAY } from "../kit/tray";
import { clamp, cubicBezier, inOutCubic, lerp, outCubic, progress, smoothstep, swiftIn, swiftInOut, swiftOut, TAU } from "../math";
import { type Caption, drawText, drawWords, font, layout, MONO, wordStyle } from "../type";
import { DASH, STRIP_BODY, STRIP as TODO_STRIP } from "./stash-peel";

const S = sec("restore");

/** Section-local seconds of beat `n`. */
const b = (n: number): number => n * BEAT;

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 3 + 4 = 7 words: need 4.5, hold 4.5 (line 2 needs 3, holds 3).
  { out: 6.5, lines: [{ in: 2, text: "Fixes are staged." }, { in: 3.5, text: "Your edits come back." }] },
  // 6 words: need 4, hold 4.
  { out: 11.5, lines: [{ in: 7.5, text: "The commit gets the fixed version." }] },
];

// The beat map, section-local seconds. The score (score/restore.ts) can
// place its cues on these.

/** The finished bars swell for a moment before they go (anticipation). */
export const T_CHARGE = b(0);
/** The warm edge sweeps the bars and tracks right to left… */
export const T_RETRACT0 = b(0.25);
/** …and slams into the file labels, which take the fixes. */
export const T_ABSORB = b(1.375);
/** The `✓ staged` stamps, one per lane on 32nds (the score's four stamps). */
export const T_STAMPS = [b(1.5), b(1.625), b(1.75), b(1.875)] as const;
/** README.md dims and its stamp flips to `= HEAD`, on the beat after the cascade. */
export const T_HEAD = b(2);
/** The divider draws down; the lanes fade. */
export const T_SPLIT = b(2);
/**
 * README.md's row (label and `= HEAD`) is the last of the lanes: it steps up
 * clear of the unfolding staged card, holds over the split's first beat, and
 * gives way to the `Staged` header. A beat on screen after the flip, so
 * `= HEAD` reads before `3 files changed` needs it.
 */
const T_HEAD_UP0 = b(2.375);
const T_HEAD_UP1 = b(2.75);
const T_HEAD_OUT0 = b(2.9375);
const T_HEAD_OUT1 = b(3.0625);
/** The src/main.py label unfolds into the staged card. */
const T_STAGED0 = b(2.3125);
const T_STAGED1 = b(2.75);
/** The worktree card draws on. */
const T_WORKTREE0 = b(2.375);
const T_WORKTREE1 = b(2.75);
/** The headers land: `Worktree` as its card fills, `Staged` once README.md has gone from under it. */
const T_WORKTREE_HEAD = b(2.875);
const T_STAGED_HEAD = b(3.1875);
/** The fixed lines slide across the divider, one row per 32nd (the score's copy whoosh). */
export const T_COPY0 = b(2.6875);
const COPY_EACH = b(1 / 32);
const COPY_DUR = b(0.25);
/** The tray opens (the score's creak) and the TODO line rises out of it. */
export const T_LID = b(3);
const T_RISE1 = b(3.25);
/** The zip: the TODO line's teeth mesh left to right (the score's zip ticks)… */
export const T_ZIP0 = T_RISE1;
export const T_ZIP1 = b(3.5);
/** …and it settles into the worktree copy with a glow (the score's bloop). */
export const T_BLOOP = T_ZIP1;
const T_LID_SHUT0 = b(3.5);
/** The lid bangs shut (the score's knock). */
export const T_LID_SHUT1 = b(3.75);
const T_TRAY_OUT0 = b(4);
const T_TRAY_OUT1 = b(4.5);
/** The split closes onto the staged side; the terminal strip drops in. */
export const T_CLOSE = b(5);
/** The cards have slid together (the end of the score's close whoosh). */
export const T_SLIDE1 = b(5.5);
const T_STRIP_IN1 = b(5.5);
/**
 * The strip lands on hk's last row, `✔ stash – Restoring unstaged changes
 * (manual)`, which it falls in showing (a puff and a terminal tick).
 */
export const T_RESTORED = T_STRIP_IN1;
/** The staged card goes into the terminal and the commit prints (a terminal tick). */
export const T_COMMITTED = b(5.75);
/** `main` draws on, left to right, with the parent commit on it (the score's line whoosh). */
export const T_LINE0 = b(5.5);
export const T_LINE1 = b(6.5);
/** `[main ada2ca4]` lifts off as a dot… */
export const T_LIFT = b(6);
const T_SHOOT0 = b(6.25);
/** …and lands at x 1400 on the beat: the reel's second bell. */
export const T_LAND = b(7);
/** The strip fades; the main line's labels come in. */
const T_STRIP_OUT0 = b(7);
const T_STRIP_OUT1 = b(8);
/** From here the frame is restore|catch, the head's glow breathing once… */
const T_STILL = b(8);
/** …and exactly restore|catch from b11, the last beat. */
const T_BREATHED = b(11);

// Stage 1: the lanes rewind and the files are stamped.

/** Starts from rest and arrives at speed: the tape measure's slam. */
const retractEase = (p: number): number => (p * p * (3 - p)) / 2;
/** The wind-up: over b0–0.25 the bars' right ends and the tracks push out this far, and the retract starts from there. */
const PUSH = 10;
const pushAt = (lt: number): number => PUSH * smoothstep(T_CHARGE, T_RETRACT0, lt);
/** The retracting edge's x: bars and tracks end here (or at their pushed ends, before it reaches them). */
const edgeAt = (lt: number): number => lerp(LANES.trackX1 + PUSH, LANES.trackX0, retractEase(progress(T_RETRACT0, T_ABSORB, lt)));
/** The edge's speed, px per second, for its smear. */
const edgeSpeed = (lt: number): number => (edgeAt(lt - 1 / 240) - edgeAt(lt + 1 / 240)) * 120;

/** The lanes, their padlocks and stamps fade as the split comes in. */
const lanesOut = (lt: number): number => smoothstep(b(2.3), b(2.8), lt);
/**
 * How far README.md's label and `= HEAD` have stepped up, px: 48 lifts the
 * chip's bottom (y 224) to 176, ahead of the unfolding card's top edge all
 * the way (at least 20 px clear, the gap left when the card stops at y 196).
 */
const HEAD_UP = 48;
const headUpAt = (lt: number): number => HEAD_UP * swiftOut(progress(T_HEAD_UP0, T_HEAD_UP1, lt));
/** README.md's row goes last, as the `Staged` header comes in where it stood. */
const headRowOut = (lt: number): number => smoothstep(T_HEAD_OUT0, T_HEAD_OUT1, lt);
/** Its padlock stays with the lanes, gone before the card's top edge could leave its shackle showing. */
const headLockOut = (lt: number): number => smoothstep(b(2.3), b(2.55), lt);
/** The other stamps are gone before the unfolding card's edge reaches them, so none pokes out beside it. */
const stampsOut = (lt: number): number => smoothstep(b(2.3), b(2.5), lt);

/** The labels flash warm as the fixes arrive in them. */
function absorbFlash(lt: number): number {
  if (lt < T_ABSORB - b(1 / 16)) return 0;
  if (lt < T_ABSORB) return progress(T_ABSORB - b(1 / 16), T_ABSORB, lt);
  return (1 - progress(T_ABSORB, T_ABSORB + b(1), lt)) ** 2;
}

const STAMP = { x: 648, h: 48, size: 32, pad: 14, check: 28, gap: 7 } as const;
type StampKind = "staged" | "head";

function stampWidth(ctx: CanvasRenderingContext2D, kind: StampKind): number {
  if (kind === "head") return STAMP.pad * 2 + monoWidth("= HEAD", STAMP.size);
  return STAMP.pad * 2 + STAMP.check + STAMP.gap + layout(ctx, "staged", font(STAMP.size, 600)).width;
}

/** A stamp chip from x, centred on cy: cyan `✓ staged` on an elevated chip, or `= HEAD` in text2 in a dashed text3 outline. */
function drawStamp(ctx: CanvasRenderingContext2D, x: number, cy: number, kind: StampKind, sx: number, sy: number, alpha: number): void {
  if (alpha <= 0 || sx <= 0 || sy <= 0.01) return;
  const w = stampWidth(ctx, kind);
  const h = STAMP.h;
  ctx.save();
  ctx.globalAlpha *= alpha;
  ctx.translate(x + w / 2, cy);
  ctx.scale(sx, sy);
  ctx.translate(-w / 2, 0);
  if (kind === "staged") {
    roundedRect(ctx, 0, -h / 2, w, h, 10);
    ctx.fillStyle = PALETTE.elevated;
    ctx.fill();
    ctx.strokeStyle = PALETTE.cyan;
    ctx.lineWidth = 2;
    ctx.stroke();
    drawDone(ctx, STAMP.pad + STAMP.check / 2, 0, { size: STAMP.size, color: PALETTE.cyan });
    drawText(ctx, "staged", STAMP.pad + STAMP.check + STAMP.gap, STAMP.size * 0.34, { font: font(STAMP.size, 600), fill: PALETTE.cyan });
  } else {
    roundedRect(ctx, 0, -h / 2, w, h, 10);
    ctx.setLineDash([6, 6]);
    ctx.strokeStyle = rgba(PALETTE.text3, 0.6);
    ctx.lineWidth = 1.5;
    ctx.stroke();
    ctx.setLineDash([]);
    drawMono(ctx, "= HEAD", STAMP.pad, STAMP.size * 0.34, STAMP.size, PALETTE.text2);
  }
  ctx.restore();
}

/** Lane `i`'s stamp: dropped from above onto the lane on its 32nd, squashed on impact, flipped on README.md; `up` px above its lane. */
function drawLaneStamp(ctx: CanvasRenderingContext2D, i: number, lt: number, alpha: number, up = 0): void {
  const hit = T_STAMPS[i];
  const fall = b(1 / 8);
  if (lt < hit - fall || alpha <= 0) return;
  const cy = LANES.rows[i] - up;
  let sx = 1;
  let sy = 1;
  let a = alpha;
  if (lt < hit) {
    // Falling toward the page: large, faint, and closing in fast.
    const p = progress(hit - fall, hit, lt);
    const s = lerp(1.55, 1, p * p);
    sx = s;
    sy = s;
    a *= progress(0, 0.7, p);
  } else {
    // Squashed on impact, then a springy settle.
    const q = land(lt, hit, 0.16, 0.35);
    sy = lerp(0.78, 1, q);
    sx = lerp(1.12, 1, q);
  }
  let kind: StampKind = "staged";
  if (i === 0 && lt >= T_HEAD) {
    // A card flip on the grid: shut, turn over, open again.
    const p = progress(T_HEAD, T_HEAD + b(1 / 8), lt);
    sy *= Math.abs(Math.cos(Math.PI * p));
    if (p >= 0.5) kind = "head";
  }
  const w = stampWidth(ctx, kind);
  if (lt >= hit && kind === "staged") {
    // The impact: a cyan glow and a rounded ring off the chip, the first loudest.
    const k = progress(hit, hit + 0.32, lt);
    if (k < 1) {
      const loud = i === 0 ? 1 : 0.65;
      ctx.save();
      ctx.globalAlpha *= alpha;
      glow(ctx, STAMP.x + w / 2, cy, 110, PALETTE.cyan, 0.45 * loud * (1 - k) ** 2);
      const grow = 22 * swiftOut(k);
      roundedRect(ctx, STAMP.x - grow, cy - STAMP.h / 2 - grow, w + 2 * grow, STAMP.h + 2 * grow, 10 + grow);
      ctx.strokeStyle = rgba(PALETTE.cyanBright, 0.8 * loud * (1 - k) ** 1.5);
      ctx.lineWidth = 3 * (1 - k) + 0.5;
      ctx.stroke();
      ctx.restore();
    }
  }
  drawStamp(ctx, STAMP.x, cy, kind, sx, sy, a);
}

/** The finished bars and their tracks, cut at the retracting edge. */
function drawRetract(ctx: CanvasRenderingContext2D, lt: number): void {
  const E = edgeAt(lt);
  const push = pushAt(lt);
  const out = lanesOut(lt);
  const out0 = headRowOut(lt);
  const up0 = headUpAt(lt);
  const flash = absorbFlash(lt);
  // Knocked left as the fixes slam in, and easing back: one push, no rattle.
  const kp = progress(T_ABSORB, T_ABSORB + 0.28, lt);
  const knock = -7 * Math.sin(Math.PI * kp) * (1 - kp);
  // Labels: drawn here, not by drawLanes, so they can flash and move.
  const headDim = smoothstep(T_HEAD, T_HEAD + b(1 / 8), lt);
  const intoCard = cardOpen(lt);
  LANE_FILES.forEach((file, i) => {
    let a = 1 - out;
    if (i === 0) a = (1 - out0) * lerp(1, 0.45, headDim);
    if (i === 2) a = 1 - progress(0, 0.3, intoCard);
    if (a <= 0) return;
    const color = mix(PALETTE.text1, PALETTE.warmBright, 0.85 * flash);
    drawMono(ctx, file, LANES.labelX + knock, LANES.rows[i] + 14 - (i === 0 ? up0 : 0), 40, rgba(color, a));
  });
  // Tracks and padlocks: the tracks push out and retract with the bars, the padlocks stay open.
  // drawLanes' own track, drawn here so it can reach past x 1760 in the wind-up.
  const trackW = Math.min(LANES.trackX1 + push, E) - LANES.trackX0;
  if (trackW > 0) {
    ctx.save();
    ctx.fillStyle = LANE.track;
    for (const cy of LANES.rows) {
      roundedRect(ctx, LANES.trackX0, cy - LANES.trackH / 2, trackW, LANES.trackH, LANES.trackRadius);
      ctx.fill();
    }
    ctx.restore();
  }
  drawLanes(ctx, { labels: 0, tracks: 0, locks: [1 - headLockOut(lt), 1 - out, 1 - out, 1 - out] });
  for (const s of SCHEDULE) {
    const x1 = Math.min(s.x1 + push, E);
    if (x1 - s.x0 < 1) continue;
    const { y0, y1 } = barSpan(s.lanes[0], s.lanes[1]);
    drawBar(ctx, { x0: s.x0, x1, y0, y1 }, "fix", { label: s.step, fullWidth: s.x1 - s.x0, rotate: s.lanes[1] - s.lanes[0] >= 2 });
  }
  // The ✔ at each lane's end swells, then rides the edge in and fades.
  const swell = 1 + 0.18 * bump(lt, T_CHARGE, T_RETRACT0 + b(0.125));
  LANES.rows.forEach((cy) => {
    const x = Math.min(1730 + push, E - 28);
    const a = progress(1080, 1420, x);
    if (a > 0) drawDone(ctx, x, cy, { size: 40, scale: swell * lerp(0.55, 1, progress(1080, 1730, x)), alpha: a });
  });
  // The edge itself while it moves: a warm cap on each lane and a smear behind it.
  const moving = progress(T_RETRACT0, T_RETRACT0 + b(1 / 8), lt) * (1 - progress(T_ABSORB, T_ABSORB + b(1 / 8), lt));
  if (moving > 0) {
    const smear = clamp(edgeSpeed(lt) / 30, 0, 260);
    ctx.save();
    LANES.rows.forEach((cy) => {
      const h = LANES.barH;
      const g = ctx.createLinearGradient(E, 0, E + smear + 1, 0);
      g.addColorStop(0, rgba(PALETTE.warm, 0.38 * moving));
      g.addColorStop(1, rgba(PALETTE.warm, 0));
      ctx.fillStyle = g;
      ctx.fillRect(E, cy - h / 2, smear + 1, h);
      roundedRect(ctx, E - 8, cy - h / 2, 8, h, 3);
      ctx.fillStyle = rgba(PALETTE.warmBright, 0.95 * moving);
      ctx.fill();
      glow(ctx, E - 4, cy, 46, PALETTE.warm, 0.5 * moving);
    });
    ctx.restore();
  }
  // The labels take the fixes: a warm bloom at each one's end.
  if (flash > 0) {
    LANE_FILES.forEach((file, i) => {
      if (i === 2 && intoCard > 0) return;
      glow(ctx, LANES.labelX + monoWidth(file, 40) + knock, LANES.rows[i], 70, PALETTE.warm, 0.35 * flash * (1 - out));
    });
  }
  // The stamps.
  LANE_FILES.forEach((_, i) => (i === 0 ? drawLaneStamp(ctx, i, lt, 1 - out0, up0) : drawLaneStamp(ctx, i, lt, (1 - out) * (1 - stampsOut(lt)))));
}

// Stage 2: the split.

const DIVIDER = { x: 960, y0: 120, y1: 700 } as const;
const HEADER_Y = 160;
const CODE = { size: 26, lineH: 32 } as const;
const TAB = "src/main.py";
const LEFT: CardRect = { x: 160, y: 196, w: 720, h: 416 };
const RIGHT: CardRect = { x: 1040, y: 196, w: 720, h: 416 };
const L_LEFT = cardLayout(LEFT, CODE);
const L_RIGHT = cardLayout(RIGHT, CODE);
/** The two cards meet here as the split closes… */
const CENTER_X = 960 - LEFT.w / 2;
/** …drifting down as they go, so the strip (y 100–264) lands clear of their tops. */
const CLOSE_DROP = 84;
/** How far the split has closed, 0..1. */
const closeAt = (lt: number): number => swiftInOut(progress(T_CLOSE, T_SLIDE1, lt));
/** The TODO line is the worktree copy's 11th row. */
const TODO_ROW = 10;
const TODO_W = monoWidth(MAIN_PY_TODO, CODE.size);

/** The divider is back up a dotted quaver after the split starts to close. */
const T_DIVIDER_UP = T_CLOSE + b(0.375);

/** The divider's reach down the frame, and how far it has gone back up. */
function drawDivider(ctx: CanvasRenderingContext2D, lt: number): void {
  const down = swiftOut(progress(T_SPLIT, b(2.75), lt));
  const up = swiftIn(progress(T_CLOSE, T_DIVIDER_UP, lt));
  if (down <= 0 || up >= 1) return;
  const end = lerp(lerp(DIVIDER.y0, DIVIDER.y1, down), DIVIDER.y0, up);
  ctx.save();
  ctx.fillStyle = mix(PALETTE.divider, PALETTE.text3, 0.35);
  ctx.fillRect(DIVIDER.x - 1, DIVIDER.y0, 2, end - DIVIDER.y0);
  // A glint at the head while it draws, and again as it goes back up.
  const head = (1 - progress(b(2.6), b(3), lt)) * progress(T_SPLIT, T_SPLIT + b(1 / 16), lt) + bump(lt, T_CLOSE, T_DIVIDER_UP - T_CLOSE);
  if (head > 0) {
    glow(ctx, DIVIDER.x, end, 34, PALETTE.cyanBright, 0.9 * head);
    const g = ctx.createLinearGradient(0, end - 60, 0, end);
    g.addColorStop(0, rgba(PALETTE.glint, 0));
    g.addColorStop(1, rgba(PALETTE.glint, 0.95 * head));
    ctx.fillStyle = g;
    ctx.fillRect(DIVIDER.x - 1.5, Math.max(DIVIDER.y0, end - 60), 3, Math.min(60, end - DIVIDER.y0));
  }
  ctx.restore();
}

const STAGED_STYLE = wordStyle(56, PALETTE.cyan);
const WORKTREE_STYLE = wordStyle(56, PALETTE.paper);

function drawHeaders(ctx: CanvasRenderingContext2D, lt: number): void {
  if (lt < T_WORKTREE_HEAD - b(1 / 8) || lt >= T_CLOSE + b(1 / 4)) return;
  drawWords(ctx, "Staged", LEFT.x, HEADER_Y, STAGED_STYLE, lt, T_STAGED_HEAD, T_CLOSE);
  drawWords(ctx, "Worktree", RIGHT.x, HEADER_Y, WORKTREE_STYLE, lt, T_WORKTREE_HEAD, T_CLOSE);
}

/** How far the staged card has unfolded out of the src/main.py label, 0..1 (linear). */
const cardOpen = (lt: number): number => progress(T_STAGED0, T_STAGED1, lt);

/** A card drawn at top-left (x, y), scale s, size w × h in card units. */
interface Placed {
  x: number;
  y: number;
  s: number;
  w: number;
  h: number;
  alpha: number;
}

// The unfold starts from the label itself: a tab-sized card at 4/3 scale
// whose tab text sits exactly on `src/main.py` (40 px, baseline 454).
const TAB_W = monoWidth(TAB, 30) + 48;
const UNFOLD_S = 40 / 30;
const UNFOLD: Placed = { x: LANES.labelX - 24 * UNFOLD_S, y: LANES.rows[2] + 14 - 29 * UNFOLD_S, s: UNFOLD_S, w: TAB_W, h: 40, alpha: 0 };

/** Where the staged card is: unfolding, at rest, sliding down to the centre, then shrinking into the terminal. */
function stagedAt(lt: number): Placed | null {
  if (lt < T_STAGED0 || lt >= T_COMMITTED) return null;
  const e = swiftInOut(cardOpen(lt));
  let p: Placed = {
    x: lerp(UNFOLD.x, LEFT.x, e),
    y: lerp(UNFOLD.y, LEFT.y, e),
    s: lerp(UNFOLD.s, 1, e),
    w: lerp(UNFOLD.w, LEFT.w, e),
    h: lerp(UNFOLD.h, LEFT.h, e),
    alpha: progress(0, 0.25, cardOpen(lt)),
  };
  const c = closeAt(lt);
  p.x += (CENTER_X - LEFT.x) * c;
  p.y += CLOSE_DROP * c;
  const k = progress(T_SLIDE1, T_COMMITTED, lt);
  if (k > 0) {
    // Into the terminal: up behind its window, toward the `[main ada2ca4]` it becomes.
    // Shrinking early, so it is small by the time it reaches the window, and never shows above it.
    const s = lerp(1, 0.28, outCubic(k));
    const m = inOutCubic(k);
    const cx = lerp(p.x + LEFT.w / 2, COMMIT_TEXT.x + 120, m);
    const cy = lerp(p.y + LEFT.h / 2, STRIP.y + STRIP.h / 2, m);
    p = { ...p, s, x: cx - (LEFT.w / 2) * s, y: cy - (LEFT.h / 2) * s, alpha: 1 - progress(0.55, 1, k) };
  }
  return p;
}

/** Where the worktree card is: drawing on in place, then sliding down under the staged card and gone. */
function worktreeAt(lt: number): Placed | null {
  const gone = T_SLIDE1 - b(1 / 16);
  if (lt < T_WORKTREE0 || lt >= gone) return null;
  const c = closeAt(lt);
  return { x: RIGHT.x + (CENTER_X - RIGHT.x) * c, y: RIGHT.y + CLOSE_DROP * c, s: 1, w: RIGHT.w, h: RIGHT.h, alpha: 1 - progress(gone - b(1 / 8), gone, lt) };
}

/** A soft drop shadow under a card, in logical px whatever the output scale. */
function cardShadow(ctx: CanvasRenderingContext2D, r: CardRect, alpha: number): void {
  if (alpha <= 0) return;
  const m = ctx.getTransform();
  const k = Math.hypot(m.a, m.b);
  ctx.save();
  ctx.shadowColor = rgba("#000000", 0.55 * alpha);
  ctx.shadowBlur = 44 * k;
  ctx.shadowOffsetY = 18 * k;
  roundedRect(ctx, r.x + 6, r.y + 6, r.w - 12, r.h - 12, 16);
  ctx.fillStyle = rgba(PALETTE.bg, alpha);
  ctx.fill();
  ctx.restore();
}

function withPlaced(ctx: CanvasRenderingContext2D, p: Placed, shadow: number, draw: (r: CardRect) => void): void {
  if (p.alpha <= 0) return;
  ctx.save();
  ctx.globalAlpha *= p.alpha;
  ctx.translate(p.x, p.y);
  ctx.scale(p.s, p.s);
  const r = { x: 0, y: 0, w: p.w, h: p.h };
  cardShadow(ctx, r, shadow);
  draw(r);
  ctx.restore();
}

function drawStagedCard(ctx: CanvasRenderingContext2D, lt: number): void {
  const p = stagedAt(lt);
  if (!p) return;
  const L = cardLayout({ x: 0, y: 0, w: p.w, h: p.h }, CODE);
  // A row shows once the unfolding card has room for all of it, not half cut.
  const lines = MAIN_PY_FIXED.map((text, row) => ({ text, alpha: progress(L.rowBottom(row) - 6, L.rowBottom(row) + 26, p.h) }));
  withPlaced(ctx, p, 1, (r) => {
    drawCard(ctx, r, { tab: TAB, lines, size: CODE.size, lineH: CODE.lineH, gutter: { color: PALETTE.cyanFill, from: 0, to: MAIN_PY_FIXED.length - 1 } });
  });
}

// The fixed lines' copies: every non-blank row, a 32nd apart.
const COPIES = MAIN_PY_FIXED.map((text, row) => ({ text, row }))
  .filter((c) => c.text.length > 0)
  .map((c, k) => ({ ...c, t0: T_COPY0 + k * COPY_EACH, t1: T_COPY0 + k * COPY_EACH + COPY_DUR }));
/** Peels off its original quickly, and eases into its row. */
const copyEase = cubicBezier(0.3, 0, 0.15, 1);
const copyX = (c: (typeof COPIES)[number], lt: number): number => lerp(L_LEFT.x0, L_RIGHT.x0, copyEase(progress(c.t0, c.t1, lt)));

function drawWorktreeCard(ctx: CanvasRenderingContext2D, lt: number): void {
  const p = worktreeAt(lt);
  if (!p) return;
  const reveal = progress(T_WORKTREE0, T_WORKTREE1, lt);
  const lines = MAIN_PY_FIXED.map((text, row) => {
    const c = COPIES.find((k) => k.row === row);
    return { text, alpha: c && lt >= c.t1 ? 1 : 0 };
  });
  withPlaced(ctx, p, progress(0.45, 1, reveal), (r) => {
    const L = drawCard(ctx, r, { tab: TAB, lines, size: CODE.size, lineH: CODE.lineH, reveal });
    if (lt >= T_ZIP1) drawTodoSettled(ctx, lt, L.x0, L.baseline(TODO_ROW), r);
  });
}

/** The TODO line at home in the worktree copy: paper, on a paper glow, with a gutter mark for an unstaged change. */
function drawTodoSettled(ctx: CanvasRenderingContext2D, lt: number, x0: number, baseline: number, r: CardRect): void {
  const top = baseline - 0.8 * CODE.size - (CODE.lineH - CODE.size) / 2;
  const bloop = (1 - progress(T_BLOOP, T_BLOOP + b(1), lt)) ** 2;
  ctx.save();
  ctx.fillStyle = rgba(PALETTE.paper, 0.07 + 0.16 * bloop);
  ctx.fillRect(r.x + 12, top, r.w - 24, CODE.lineH);
  const mark = land(lt, T_BLOOP, 0.2, 0.4);
  ctx.fillStyle = PALETTE.paper;
  ctx.fillRect(r.x + 12, top + (CODE.lineH / 2) * (1 - mark), 6, CODE.lineH * mark);
  if (bloop > 0) glow(ctx, x0 + TODO_W / 2, top + CODE.lineH / 2, 150, PALETTE.paper, 0.35 * bloop);
  ctx.restore();
  drawMono(ctx, MAIN_PY_TODO, x0, baseline, CODE.size, PALETTE.paper);
}

/** The copies in flight: warm while they cross, smeared, and a flash on the divider as each goes over it. */
function drawCopies(ctx: CanvasRenderingContext2D, lt: number): void {
  for (const c of COPIES) {
    if (lt <= c.t0 || lt >= c.t1) continue;
    const x = copyX(c, lt);
    const v = x - copyX(c, lt - 1 / 120);
    const y = L_LEFT.baseline(c.row);
    const w = monoWidth(c.text, CODE.size);
    const heat = bump(lt, c.t0, COPY_DUR);
    const color = mix(PALETTE.text1, PALETTE.warm, 0.9 * heat);
    // Faint while it still lies over the original, so the two never read as one smudged line.
    const a = progress(L_LEFT.x0, L_LEFT.x0 + 0.6 * w + 40, x);
    ctx.save();
    // A warm streak trailing it at speed: motion blur, not a second copy of the text.
    const streak = clamp(Math.abs(v) * 7, 0, 420);
    if (streak > 20) {
      const band = { y: y - 0.62 * CODE.size, h: 0.62 * CODE.size };
      const g = ctx.createLinearGradient(x - streak, 0, x + Math.min(w, 120), 0);
      g.addColorStop(0, rgba(PALETTE.warm, 0));
      g.addColorStop(1, rgba(PALETTE.warm, 0.2 * a * clamp((streak - 20) / 120)));
      ctx.fillStyle = g;
      ctx.fillRect(x - streak, band.y, streak + Math.min(w, 120), band.h);
    }
    drawMono(ctx, c.text, x, y, CODE.size, rgba(color, a));
    // Crossing the divider.
    const mid = x + w / 2;
    const near = clamp(1 - Math.abs(mid - DIVIDER.x) / 260);
    if (near > 0) {
      const cy = y - 0.33 * CODE.size;
      glow(ctx, DIVIDER.x, cy, 60, PALETTE.cyanBright, 0.7 * near);
      ctx.fillStyle = rgba(PALETTE.glint, 0.8 * near);
      ctx.fillRect(DIVIDER.x - 1.5, cy - 22, 3, 44);
    }
    ctx.restore();
  }
}

// The tray and the TODO line.

/** How far the lid opens here (the kit's 1 is −35°). */
const LID_OPEN = 0.5;

function drawTrayLayer(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  const a = 1 - smoothstep(T_TRAY_OUT0, T_TRAY_OUT1, lt);
  if (a <= 0) return;
  const lid = LID_OPEN * swiftOut(progress(T_LID, T_LID + b(1 / 8), lt)) * (1 - swiftIn(progress(T_LID_SHUT0, T_LID_SHUT1, lt)));
  // A little knock as the lid bangs shut.
  const dy = 2.5 * jolt(lt, T_LID_SHUT1, 0.18, 12);
  ctx.save();
  // The lid swings up behind the worktree card, never across it and the zip: the tray shows only below the card.
  const card = worktreeAt(lt);
  if (card && lid > 0) {
    ctx.beginPath();
    ctx.rect(0, card.y + card.h * card.s, 1920, 1080);
    ctx.clip();
  }
  ctx.translate(0, dy);
  drawTray(ctx, { lid, holding: lt < T_LID, t: env.t, alpha: a });
  ctx.restore();
}

/** The slot the held line shows in, where the TODO strip starts from. */
const SLOT = TRAY.sliver;
const SLOT_C: Pt = { x: (SLOT.x0 + SLOT.x1) / 2, y: (SLOT.y0 + SLOT.y1) / 2 };
/** The tray's sliver colour (tray.ts): the strip is still this as it leaves the slot. */
const SLIVER = mix(PALETTE.paper, PALETTE.paperDim, 0.35);
/**
 * The strip is the one stash cut off its card (scenes/stash-peel.ts): the
 * same size, paper, radius and dashed outline, with the line 14 px in on a
 * baseline 25.8 px down (its TEXT_DX and TEXT_DY). At home its line sits on
 * the worktree copy's row 10, as the stash strip lay on its card's.
 */
const STRIP_TEXT = { dx: 14, dy: 25.8 } as const;
const TODO_HOME: Pt = {
  x: L_RIGHT.x0 - STRIP_TEXT.dx + TODO_STRIP.w / 2,
  y: L_RIGHT.baseline(TODO_ROW) - STRIP_TEXT.dy + TODO_STRIP.h / 2,
};
/** The line's first glyph and its baseline, from the strip's centre. */
const TEXT_X = STRIP_TEXT.dx - TODO_STRIP.w / 2;
const TEXT_BASE = STRIP_TEXT.dy - TODO_STRIP.h / 2;
/** The row's middle, from the strip's centre. */
const ROW_DY = L_RIGHT.rowTop(TODO_ROW) + CODE.lineH / 2 - TODO_HOME.y;
/** A low arc up out of the tray and left into the row, under the rows above it. */
const RISE_C: Pt = { x: SLOT_C.x - 60, y: TODO_HOME.y - 60 };
/**
 * The open zipper's teeth: the glyphs sit alternately low and high on the
 * strip, px from the line's baseline for even and odd columns, and meet on
 * it as they mesh. Both stay inside the outline (no low glyph has a descender).
 */
const TEETH = [5, -2] as const;
/** The settled line's band runs the card's width, as an editor marks a changed line. */
const BAND = { x0: RIGHT.x + 12, x1: RIGHT.x + RIGHT.w - 12 } as const;
/** Up out of the slot slowly, over fast, and eased into the row. */
const riseEase = cubicBezier(0.45, 0, 0.2, 1);

const quad = (a: Pt, c: Pt, e: Pt, u: number): Pt => ({
  x: (1 - u) ** 2 * a.x + 2 * (1 - u) * u * c.x + u * u * e.x,
  y: (1 - u) ** 2 * a.y + 2 * (1 - u) * u * c.y + u * u * e.y,
});

/**
 * The TODO line from the tray: the held sliver lifts out of the slot and
 * unfolds into the stash's strip as it arcs over (the reverse of how it went
 * in: scaled up and turned face-on), its glyphs apart like an open zipper's
 * teeth. Then a slider runs along the row and the teeth mesh behind it, and
 * the row's band lights up: it is part of the file.
 */
function drawTodoStrip(ctx: CanvasRenderingContext2D, lt: number): void {
  if (lt < T_LID || lt >= T_ZIP1) return;
  const u = riseEase(progress(T_LID, T_RISE1, lt));
  const pos = quad(SLOT_C, RISE_C, TODO_HOME, u);
  const { w, h, r } = TODO_STRIP;
  const sx = lerp((SLOT.x1 - SLOT.x0) / w, 1, u);
  const sy = lerp((SLOT.y1 - SLOT.y0) / h, 1, u);
  const tilt = -0.24 * Math.sin(Math.PI * u);
  const adv = 0.6 * CODE.size;
  // The zip: the slider's column, from the band's left end to its right end.
  const zp = progress(T_ZIP0, T_ZIP1, lt);
  const col0 = (BAND.x0 - L_RIGHT.x0) / adv - 0.5;
  const col1 = (BAND.x1 - L_RIGHT.x0) / adv + 0.5;
  const slider = lerp(col0, col1, smoothstep(0, 1, zp));
  const sliderX = TEXT_X + slider * adv;
  ctx.save();
  if (zp > 0) {
    // Merged behind the slider: the settled row's band, lit.
    const top = L_RIGHT.rowTop(TODO_ROW);
    ctx.fillStyle = rgba(PALETTE.paper, 0.23);
    ctx.fillRect(BAND.x0, top, clamp(TODO_HOME.x + sliderX - BAND.x0, 0, BAND.x1 - BAND.x0), CODE.lineH);
  }
  ctx.translate(pos.x, pos.y);
  ctx.rotate(tilt);
  ctx.scale(sx, sy);
  // Ahead of the slider: the strip as it went into the stash (drawStripFace's paper and outline).
  ctx.save();
  ctx.beginPath();
  ctx.rect(sliderX, -h, 2 * w, 2 * h);
  ctx.clip();
  roundedRect(ctx, -w / 2, -h / 2, w, h, r);
  ctx.fillStyle = STRIP_BODY;
  ctx.fill();
  const outline = progress(0.15, 0.5, u);
  if (outline > 0) {
    roundedRect(ctx, -w / 2 + 1, -h / 2 + 1, w - 2, h - 2, r - 1);
    ctx.setLineDash([...DASH]);
    ctx.strokeStyle = rgba(PALETTE.paper, outline);
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.setLineDash([]);
  }
  ctx.restore();
  // The glyphs: teeth apart, alternately low and high, meshing as the slider passes.
  const ga = progress(0.1, 0.5, u);
  if (ga > 0) {
    Array.from(MAIN_PY_TODO).forEach((ch, c) => {
      if (ch === " ") return;
      const z = progress(c - 0.25, c + 1.5, slider);
      const open = 1 - land(z, 0, 1, 0.4);
      drawMono(ctx, ch, TEXT_X + c * adv, TEXT_BASE + TEETH[c % 2] * open, CODE.size, rgba(PALETTE.paper, ga * lerp(1, 0.8, open)));
    });
  }
  // Leaving the slot it is still the tray's sliver, turning to paper as it unfolds.
  const sliver = 1 - progress(0, 0.35, u);
  if (sliver > 0) {
    roundedRect(ctx, -w / 2, -h / 2, w, h, r);
    ctx.fillStyle = rgba(SLIVER, sliver);
    ctx.fill();
  }
  // The row starts to glow as the last teeth mesh: the bloop's light, rising into it.
  const bloom = progress(0.7, 1, zp);
  if (bloom > 0) glow(ctx, 0, ROW_DY, 150, PALETTE.paper, 0.35 * bloom * bloom);
  // The slider: a bright pull riding the row.
  const pull = progress(0, 0.06, zp) * (1 - progress(0.9, 1, zp));
  if (pull > 0) {
    glow(ctx, sliderX, ROW_DY, 44, PALETTE.paper, 0.9 * pull);
    roundedRect(ctx, sliderX - 5, -h / 2 - 3, 10, h + 6, 5);
    ctx.fillStyle = rgba(PALETTE.glint, pull);
    ctx.fill();
  }
  ctx.restore();
}

// Stage 3: the commit.

/** The strip's lines: commit frame 22, of which it shows the bottom two as it scrolls. */
const STRIP_LINES = commit[22];
/** Row 0 after the commit prints: `[main ada2ca4]`'s 14 cells, where the dot lifts off. */
const HASH_COLS = "[main ada2ca4]".length;
const STRIP_REST = termLayout(STRIP, STRIP_LINES.length, 10);
const COMMIT_TEXT: Pt = { x: STRIP_REST.col(HASH_COLS / 2), y: STRIP_REST.baseline(10) - 0.33 * STRIP.size };

interface StripState {
  pane: Pane;
  scroll: number;
  alpha: number;
}

/**
 * The terminal strip: drops in from above showing frame 19's last two rows
 * (`✔ newlines`, `✔ stash – Restoring unstaged changes (manual)`), lands,
 * scrolls two rows as the commit prints (frame 22), then fades.
 */
function stripAt(lt: number): StripState | null {
  if (lt < T_CLOSE || lt >= T_STRIP_OUT1) return null;
  // It falls, landing on the restore's beat, and hops once.
  const fall = progress(T_CLOSE + b(1 / 8), T_STRIP_IN1, lt);
  const hop = progress(T_STRIP_IN1, T_STRIP_IN1 + b(3 / 8), lt);
  const leave = smoothstep(T_STRIP_OUT0, T_STRIP_OUT1, lt);
  const dy = -290 * (1 - fall * fall) - 16 * Math.sin(Math.PI * hop) * (1 - hop) - 16 * leave;
  const scroll = 8 + 2 * swiftOut(progress(T_COMMITTED, T_COMMITTED + b(1 / 8), lt));
  return { pane: { ...STRIP, y: STRIP.y + dy, baseline0: STRIP.baseline0 + dy }, scroll, alpha: 1 - leave };
}

function drawStrip(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void {
  const s = stripAt(lt);
  if (!s || s.alpha <= 0) return;
  const L = drawTerm(ctx, s.pane, STRIP_LINES, { t: env.t, scroll: s.scroll, alpha: s.alpha });
  ctx.save();
  ctx.globalAlpha *= s.alpha;
  // The restore row's ✔ pops as the strip lands.
  const k = progress(T_RESTORED, T_RESTORED + 0.4, lt);
  if (k > 0 && k < 1) {
    const cell = L.cell(9, 0);
    ring(ctx, cell.x + cell.w / 2, L.baseline(9) - 0.33 * STRIP.size, 34, k, PALETTE.green, 3);
    glow(ctx, cell.x + cell.w / 2, L.baseline(9) - 0.33 * STRIP.size, 50, PALETTE.green, 0.5 * (1 - k) ** 2);
  }
  // The commit's row flashes cyan as it prints.
  const f = progress(T_COMMITTED, T_COMMITTED + 0.45, lt);
  if (f > 0 && f < 1) {
    const top = L.baseline(10) - 0.8 * STRIP.size - 6;
    ctx.fillStyle = rgba(PALETTE.cyan, 0.22 * (1 - f) ** 2);
    ctx.fillRect(L.body.x + 8, top, L.body.w - 16, STRIP.lineH);
    glow(ctx, COMMIT_TEXT.x, COMMIT_TEXT.y + s.pane.y - STRIP.y, 150, PALETTE.cyan, 0.5 * (1 - f) ** 2);
  }
  ctx.restore();
}

// The dot's flight: drawn off the text into the window's left gutter, down
// the gutter clear of the rows, onto `main`, and along it to the head's place.

const LIFT_TO: Pt = { x: (STRIP.x + STRIP.x0) / 2, y: COMMIT_TEXT.y - 16 };
const TOUCH: Pt = { x: 640, y: MAIN.y };
/** Straight down out of the window first, then turning onto the line. */
const FALL_C: Pt = { x: LIFT_TO.x, y: MAIN.y };
const HEAD: Pt = { x: MAIN.head.x, y: MAIN.y };
/** The fall's arc, sampled by length, then the straight run along the line. */
const FALL = (() => {
  const n = 64;
  const pts: Pt[] = [];
  const len: number[] = [0];
  for (let i = 0; i <= n; i++) {
    pts.push(quad(LIFT_TO, FALL_C, TOUCH, i / n));
    if (i > 0) len.push(len[i - 1] + Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y));
  }
  return { pts, len, total: len[n] };
})();
const RUN = HEAD.x - TOUCH.x;
const PATH_LEN = FALL.total + RUN;

/** A point `d` px along the dot's path. */
function pathAt(d: number): Pt {
  if (d >= FALL.total) return { x: TOUCH.x + Math.min(RUN, d - FALL.total), y: MAIN.y };
  const { pts, len } = FALL;
  let i = 1;
  while (i < len.length - 1 && len[i] < d) i++;
  const k = (d - len[i - 1]) / (len[i] - len[i - 1] || 1);
  return { x: lerp(pts[i - 1].x, pts[i].x, k), y: lerp(pts[i - 1].y, pts[i].y, k) };
}

/** Starts from the hover and arrives flat out: it shoots. */
const shootEase = (p: number): number => (p * p * (3 - p)) / 2;
const dotAt = (lt: number): Pt => pathAt(PATH_LEN * shootEase(progress(T_SHOOT0, T_LAND, lt)));

/** `[main ada2ca4]` glowing cyan over the terminal's own text, then drawn together into a dot in the gutter to its left, rising a little. */
function drawLift(ctx: CanvasRenderingContext2D, lt: number): void {
  if (lt < T_LIFT || lt >= T_SHOOT0) return;
  const p = progress(T_LIFT, T_SHOOT0, lt);
  const heat = progress(0, 0.35, p);
  const pull = swiftIn(progress(0.3, 1, p));
  const text = "[main ada2ca4]";
  const size = STRIP.size;
  const adv = 0.6 * size;
  const y = STRIP_REST.baseline(10);
  ctx.save();
  glow(ctx, lerp(COMMIT_TEXT.x, LIFT_TO.x, pull), COMMIT_TEXT.y, 170 * (1 - 0.6 * pull), PALETTE.cyan, 0.55 * heat * (1 - progress(0.55, 1, p)));
  Array.from(text).forEach((ch, j) => {
    const x = STRIP_REST.col(j) + adv / 2;
    const gx = lerp(x, LIFT_TO.x, pull);
    const gy = lerp(y - 0.33 * size, LIFT_TO.y, pull);
    const s = lerp(size, size * 0.3, pull);
    const a = heat * (1 - progress(0.7, 1, p));
    drawMono(ctx, ch, gx - 0.3 * s, gy + 0.33 * s, s, rgba(mix(PALETTE.text1, PALETTE.cyan, heat), a), 700);
  });
  const r = 10 * progress(0.55, 1, p);
  if (r > 0) {
    const c = { x: LIFT_TO.x, y: lerp(COMMIT_TEXT.y, LIFT_TO.y, pull) };
    glow(ctx, c.x, c.y, r * 6, PALETTE.cyan, 0.8);
    ctx.fillStyle = mix(PALETTE.cyan, "#ffffff", 0.5);
    ctx.beginPath();
    ctx.arc(c.x, c.y, r, 0, TAU);
    ctx.fill();
  }
  ctx.restore();
}

/** The dot in flight, with its comet tail, and the line lit behind it. */
function drawShot(ctx: CanvasRenderingContext2D, lt: number): void {
  if (lt < T_SHOOT0 || lt >= T_LAND + 0.3) return;
  ctx.save();
  // The run along the line lights it, fading once the dot is home.
  const at = dotAt(Math.min(lt, T_LAND));
  const fade = 1 - progress(T_LAND, T_LAND + 0.3, lt);
  if (at.x > TOUCH.x && at.y >= MAIN.y - 0.5) {
    const g = ctx.createLinearGradient(at.x - 520, 0, at.x, 0);
    g.addColorStop(0, rgba(PALETTE.cyan, 0));
    g.addColorStop(1, rgba(PALETTE.cyan, 0.85 * fade));
    ctx.fillStyle = g;
    ctx.fillRect(Math.max(TOUCH.x, at.x - 520), MAIN.y - 2.5, Math.min(520, at.x - TOUCH.x), 5);
  }
  if (lt < T_LAND) {
    // The tail: where it was over the last few frames.
    ctx.lineCap = "butt";
    const n = 20;
    for (let k = n; k >= 1; k--) {
      const p0 = dotAt(lt - k / 280);
      const p1 = dotAt(lt - (k - 1) / 280);
      const f = 1 - k / (n + 1);
      ctx.strokeStyle = rgba(PALETTE.cyan, 0.85 * f);
      ctx.lineWidth = 18 * f;
      ctx.beginPath();
      ctx.moveTo(p0.x, p0.y);
      ctx.lineTo(p1.x, p1.y);
      ctx.stroke();
    }
    const p = dotAt(lt);
    glow(ctx, p.x, p.y, 70, PALETTE.cyan, 0.9);
    ctx.fillStyle = mix(PALETTE.cyan, "#ffffff", 0.5);
    ctx.beginPath();
    ctx.arc(p.x, p.y, 10, 0, TAU);
    ctx.fill();
  }
  ctx.restore();
}

/** The bell: a flash, rings and rays off the head as it lands, and a shock running both ways along the line. */
function drawBell(ctx: CanvasRenderingContext2D, lt: number): void {
  if (lt < T_LAND || lt >= T_LAND + 0.5) return;
  const d = lt - T_LAND;
  ctx.save();
  const k = progress(0, 0.42, d);
  glow(ctx, HEAD.x, HEAD.y, 130, PALETTE.cyan, 0.95 * (1 - k) ** 2);
  ring(ctx, HEAD.x, HEAD.y, 120, progress(0, 0.3, d), PALETTE.glint, 5);
  ring(ctx, HEAD.x, HEAD.y, 74, progress(0.04, 0.28, d), PALETTE.cyan, 4);
  // Sparks thrown off the strike: short dashes flying out and fading.
  const e = swiftOut(progress(0, 0.26, d));
  const ra = (1 - progress(0, 0.26, d)) ** 2;
  if (ra > 0) {
    ctx.strokeStyle = rgba(mix(PALETTE.cyan, PALETTE.glint, 0.5), 0.95 * ra);
    ctx.lineWidth = 3;
    ctx.lineCap = "round";
    for (let i = 0; i < 8; i++) {
      const th = (i / 8) * TAU + Math.PI / 8;
      const r1 = 26 + 92 * e;
      const r0 = r1 - 18 * (1 - e) - 4;
      ctx.beginPath();
      ctx.moveTo(HEAD.x + Math.cos(th) * r0, HEAD.y + Math.sin(th) * r0);
      ctx.lineTo(HEAD.x + Math.cos(th) * r1, HEAD.y + Math.sin(th) * r1);
      ctx.stroke();
    }
  }
  // The shock along the line, both ways.
  const sa = 1 - progress(0, 0.36, d);
  if (sa > 0) {
    for (const dir of [-1, 1]) {
      const x = HEAD.x + dir * 1500 * d;
      if (x < MAIN.x0 || x > MAIN.x1) continue;
      glow(ctx, x, MAIN.y, 30, PALETTE.cyanBright, 0.8 * sa);
    }
  }
  ctx.restore();
}

/** `main`: drawn on left to right with the parent commit on it, the head landing, then the labels. */
function drawMainStage(ctx: CanvasRenderingContext2D, lt: number): void {
  if (lt < T_LINE0) return;
  const line = swiftOut(progress(T_LINE0, T_LINE1, lt));
  const tip = lerp(MAIN.x0, MAIN.x1, line);
  ctx.save();
  ctx.fillStyle = PALETTE.text3;
  ctx.fillRect(MAIN.x0, MAIN.y - MAIN.width / 2, tip - MAIN.x0, MAIN.width);
  // The tip's glow grows in as it leaves x 160, so no blob waits there on the first frame.
  if (line < 1) glow(ctx, tip, MAIN.y, 40, PALETTE.cyan, 0.6 * (1 - line) * progress(T_LINE0, T_LINE0 + b(1 / 8), lt));
  // The parent was already there: it pops in as the line starts and nods when the shock reaches it.
  const shockAt = T_LAND + (HEAD.x - MAIN.parent.x) / 1500;
  const pr = MAIN.dotR * land(lt, T_LINE0, 0.22, 0.3) * (1 + 0.18 * bump(lt, shockAt, 0.16));
  drawCommitDot(ctx, MAIN.parent.x, "parent", { r: pr });
  if (lt >= T_LAND) {
    // Squashed along the line on impact, then round again.
    const q = progress(T_LAND, T_LAND + b(1 / 2), lt);
    const w = q < 1 ? Math.cos(TAU * 4 * (lt - T_LAND)) * (1 - q) ** 2 : 0;
    const sx = 1 + 0.4 * w;
    ctx.save();
    ctx.translate(HEAD.x, HEAD.y);
    ctx.scale(sx, 1 / sx);
    ctx.translate(-HEAD.x, -HEAD.y);
    drawCommitDot(ctx, HEAD.x, "head");
    ctx.restore();
  }
  // The labels: `ada2ca4` and its message rise over the head, `main` slides in, the parent's hash drops in under it.
  const mono = font(40, 400, MONO);
  const rise = (from: number, to: number) => {
    const a = progress(from, to, lt);
    return { a, off: 1 - swiftOut(a) };
  };
  const h = rise(b(7.25), b(7.5));
  const m = rise(b(7.25), b(7.625));
  const l = rise(b(7.25), b(7.75));
  const f = rise(b(7.375), b(7.875));
  if (l.a > 0) drawText(ctx, MAIN.label.text, MAIN.label.x - 24 * l.off, MAIN.label.y, { font: mono, fill: rgba(PALETTE.cyan, l.a) });
  if (f.a > 0) drawText(ctx, MAIN.parent.hash, MAIN.parent.x, MAIN.parent.hashY - 14 * f.off, { font: mono, fill: rgba(PALETTE.text2, f.a), align: "center" });
  if (h.a > 0) drawText(ctx, MAIN.head.hash, HEAD.x, MAIN.head.hashY + 16 * h.off, { font: mono, fill: rgba(PALETTE.text1, h.a), align: "center" });
  if (m.a > 0) drawText(ctx, MAIN.head.message, HEAD.x, MAIN.head.messageY + 16 * m.off, { font: font(40, 500), fill: rgba(PALETTE.text2, m.a), align: "center" });
  ctx.restore();
}

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    if (lt >= T_STILL) {
      // The hold breathes: the head's glow swells once and settles, a sin² that is exactly 0 from b11.
      const breath = bump(lt, T_STILL, T_BREATHED - T_STILL);
      if (breath > 0) {
        bg(ctx, env);
        drawMain(ctx, { headGlow: 1 + 0.6 * breath });
      } else drawHandoff(ctx, "restore|catch", env);
      return;
    }
    bg(ctx, env);
    if (lt < T_HEAD_OUT1) drawRetract(ctx, lt);
    drawDivider(ctx, lt);
    drawHeaders(ctx, lt);
    drawWorktreeCard(ctx, lt);
    drawStagedCard(ctx, lt);
    drawCopies(ctx, lt);
    drawTrayLayer(ctx, lt, env);
    drawTodoStrip(ctx, lt);
    drawMainStage(ctx, lt);
    drawStrip(ctx, lt, env);
    drawLift(ctx, lt);
    drawShot(ctx, lt);
    drawBell(ctx, lt);
  },
  // The terminal strip is the lit screen while it is up (b5–8), as much of it as is on screen as it falls in.
  lit(lt) {
    const s = stripAt(lt);
    const a = s ? s.alpha * clamp((s.pane.y + s.pane.h) / s.pane.h) : 0;
    return s && a > 0 ? termLit(s.pane, a) : null;
  },
  captions: () => CAPTIONS,
};
