// Scene 11, "mise use hk" (storyboard §6.11): the end card. The wordmark
// morph built is missing only its barb; on the downbeat the barb clicks out
// of the point, a glint runs down the leg into a sparkle on the point, and
// the leg swings on its root while a cyan backlight blooms behind the mark
// and settles into a faint halo. Then the card's copy arrives down the
// left, on the social card's composition (text at x 160, the logo on the
// right, warm paper type on a cool ground, cyan the only accent): "hk",
// the tagline word by word, the install box with `mise use hk` typed into
// it, and the address. One small reaction swing (and a faint glint) answers
// the closing heave-ho, and from b10 nothing moves: the last frame is the
// card, whole and still. As the bloom comes up the card is returned as the
// lit screen, so the vignette eases off the mark and the copy.
//
// Every value is a function of local time (and the cursor's blink of global
// time), so the frame at lt 0 is morph|end exactly: every effect below is
// zero there and the barb has not started.

import { BEAT, PALETTE, type Scene, sec, TERM } from "../bible";
import { mix, rgba } from "../color";
import { glow, makeCanvas, ring, roundedRect } from "../fx";
import { drawHandoff } from "../handoff";
import { drawLogo, glint, LOGO_END, LOGO_POINT, logoScale, LOGO_OPEN, logoToPx, settledSwing, sparkle } from "../kit/logo";
import { cursorOn } from "../kit/motion";
import { clamp, inOutSine, lerp, outCubic, progress, smoothstep, swiftOut } from "../math";
import { type Caption, drawText, drawWords, font, layout, MONO, WORD, type WordStyle, wordStyle } from "../type";

const S = sec("end");

/** Local seconds of section beat `n`. */
const b = (n: number): number => n * BEAT;

/** The section's must-read captions (none: the card is the copy). */
export const CAPTIONS: readonly Caption[] = [];

// Cues, local seconds. The score (score/end.ts) lands on these beats.

/** The barb clicks out of the point: b0–0.125. */
export const T_CLICK = 0;
export const BARB_DUR = b(0.125);
/** The glint runs root to point over b0–0.75, into the sparkle's peak. */
export const GLINT_DUR = b(0.75);
export const T_SPARKLE = b(0.75);
/** "hk" has risen. */
export const T_HK = b(1);
/** The tagline's last word lands; its first starts to rise a 32nd per word earlier (b2). */
export const T_TAGLINE = b(2.75);
/** The install box draws in. */
export const BOX_IN = [b(3.5), b(4)] as const;
/** The prompt's `$` and the cursor. */
export const T_PROMPT = b(3.75);
/** One key per 32nd from b3.875: the last, `k`, lands on b5.125. */
export const T_KEY0 = b(3.875);
export const KEY_EVERY = b(0.125);
/** The address has risen. */
export const T_URL = b(6);
/** The reaction swing, on the heave-ho's first boot. */
export const T_REACT = b(8);
/** The cursor goes out, on the heave-ho's hands. */
export const CURSOR_OFF = b(9);
/** Nothing moves from here. */
export const STILL = b(10);

// The copy (facts.md §0: `docs/index.md:4`, `HomePage.vue:4`, `config.mts:18`).
export const NAME = "hk";
export const TAGLINE = "Git hooks for linters and formatters";
export const PROMPT = "$ ";
export const INSTALL = "mise use hk";
export const URL_TEXT = "hk.jdx.dev";

// Layout (storyboard §6.11).
const LEFT = 160;
/**
 * The name stands on the wordmark's line: its baseline is the ink bottom of
 * the logo's h and k stems (y 80 plus the round cap, ≈ 428), not the
 * storyboard's 400, so the two "hk"s sit side by side on one line.
 */
const NAME_Y = logoToPx(LOGO_END, 28, 85).y;
const NAME_SIZE = 220;
const NAME_FONT = font(NAME_SIZE, 700);
/** −0.04 em, laid out per glyph. */
const NAME_TRACK = -0.04 * NAME_SIZE;
const TAGLINE_Y = 600;
/** The tagline is 72 px unless that would run past 1360 px (64 px then). */
const TAGLINE_MAX = 1360;
export const BOX = { x: 160, y: 660, w: 600, h: 110, r: 12 } as const;
const MONO_SIZE = 64;
const PROMPT_X = 200;
const INSTALL_Y = 735;
const URL_Y = 880;
const URL_STYLE: WordStyle = { ...wordStyle(64, PALETTE.cyan), font: font(64, 500) };

// The mark.

/**
 * The leg's swing, degrees: the click's `swingDeg(t, b0, 6°, 1 beat)`
 * calmed over b2–3, and the reaction's 3° from b8 calmed over b9–10. Both
 * are exactly 0 outside their spans (settledSwing), so the first frame and
 * everything from b10 are the mark at rest.
 */
export function swingAt(lt: number): number {
  return settledSwing(lt, T_CLICK, 6, BEAT, [b(2), b(3)]) + settledSwing(lt, T_REACT, 3, BEAT, [b(9), b(10)]);
}

/** The barb's progress: out of the point in a 32nd, fast then soft. */
const barbAt = (lt: number): number => outCubic(progress(T_CLICK, T_CLICK + BARB_DUR, lt));

/** The sparkle is scaled with the mark: kit sparkles are sized for LOGO_OPEN. */
const SPARKLE_SCALE = logoScale(LOGO_END) / logoScale(LOGO_OPEN);

// A big soft light behind the mark. glow()'s 128 px sprite bands when it is
// stretched this far, so the backlight has its own larger sprite with a
// Gaussian falloff.
let backSprite: HTMLCanvasElement | null = null;
function backlight(ctx: CanvasRenderingContext2D, x: number, y: number, r: number, color: string, alpha: number): void {
  if (!(alpha > 0)) return;
  if (!backSprite) {
    const n = 512;
    backSprite = makeCanvas(n, n);
    const g = backSprite.getContext("2d")!;
    const grad = g.createRadialGradient(n / 2, n / 2, 0, n / 2, n / 2, n / 2);
    for (let i = 0; i <= 24; i++) {
      const k = i / 24;
      grad.addColorStop(k, rgba("#ffffff", Math.exp(-4 * k * k) * (1 - k * k)));
    }
    g.fillStyle = grad;
    g.fillRect(0, 0, n, n);
  }
  ctx.save();
  // Tinted through a second pass: the white sprite masks a colour fill.
  ctx.globalCompositeOperation = "lighter";
  ctx.globalAlpha *= clamp(alpha);
  const tint = tinted(color);
  ctx.drawImage(tint, x - r, y - r, r * 2, r * 2);
  ctx.restore();
}
const tints = new Map<string, HTMLCanvasElement>();
function tinted(color: string): HTMLCanvasElement {
  let c = tints.get(color);
  if (c) return c;
  const src = backSprite!;
  c = makeCanvas(src.width, src.height);
  const g = c.getContext("2d")!;
  g.drawImage(src, 0, 0);
  g.globalCompositeOperation = "source-in";
  g.fillStyle = color;
  g.fillRect(0, 0, c.width, c.height);
  tints.set(color, c);
  return c;
}

/** The mark's optical middle, px: the backlight's centre. */
const MARK_MID = logoToPx(LOGO_END, 80, 62);

/**
 * The backlight's strength: it blooms on the click, falls back over two
 * beats and rests as a faint halo, the social card's glow behind the logo.
 */
function bloomAt(lt: number): number {
  if (lt <= 0) return 0;
  const rise = outCubic(progress(0, b(0.25), lt));
  const fall = smoothstep(b(0.25), b(2), lt);
  return rise * lerp(1, HALO, fall);
}
/** The halo's share of the bloom, at rest. */
const HALO = 0.3;

function drawMark(ctx: CanvasRenderingContext2D, lt: number): void {
  const swing = swingAt(lt);
  const point = logoToPx(LOGO_END, LOGO_POINT[0], LOGO_POINT[1], swing);

  // Light behind the mark: a wide deep teal wash and a tighter cyan core on the click.
  const bloom = bloomAt(lt);
  backlight(ctx, MARK_MID.x, MARK_MID.y, 620, PALETTE.cyanDeep, 0.5 * bloom);
  const core = lt <= 0 ? 0 : outCubic(progress(0, b(0.125), lt)) * (1 - smoothstep(b(0.125), b(1.5), lt));
  backlight(ctx, MARK_MID.x, MARK_MID.y, 380, PALETTE.logo, 0.28 * core);

  drawLogo(ctx, LOGO_END, [1, 1, 1, 1, 1, barbAt(lt)], { swing });

  // The click: a hot point and a ring off it, gone within a beat.
  if (lt > 0 && lt < b(1)) {
    const hot = outCubic(progress(0, b(0.0625), lt)) * (1 - smoothstep(b(0.0625), b(0.75), lt));
    glow(ctx, point.x, point.y, 120, PALETTE.glint, 0.7 * hot);
    ctx.save();
    ctx.globalAlpha *= 0.55;
    ring(ctx, point.x, point.y, 150, progress(0, b(0.5), lt), PALETTE.logo, 4);
    ctx.restore();
  }
  glint(ctx, LOGO_END, inOutSine(progress(T_CLICK, T_CLICK + GLINT_DUR, lt)), { swing });
  // The reaction's glint: fainter, down the leg as it swings on the boot.
  glint(ctx, LOGO_END, inOutSine(progress(T_REACT, T_REACT + GLINT_DUR, lt)), { swing, alpha: 0.55 });

  const u = progress(T_SPARKLE - b(0.25), T_SPARKLE + b(0.25), lt);
  if (u > 0 && u < 1) {
    ctx.save();
    ctx.translate(point.x, point.y);
    ctx.scale(SPARKLE_SCALE, SPARKLE_SCALE);
    sparkle(ctx, 0, 0, u);
    ctx.restore();
  }
}

// The copy.

/**
 * "hk", set per glyph at −0.04 em. Each glyph rises and fades in the way
 * the reel's words do (type.ts drawWords), one per 32nd, the k landing on
 * b1, with a little overshoot that settles.
 */
function drawName(ctx: CanvasRenderingContext2D, lt: number): void {
  const n = NAME.length;
  const dur = 1.5 * WORD;
  const start = (i: number) => T_HK - dur - (n - 1 - i) * WORD;
  if (lt < start(0)) return;
  // Optical margin: the h's stem, not its sidebearing, sits on the column's edge.
  const x = LEFT - nameBearing(ctx);
  const rise = 64;
  drawText(ctx, NAME, x, NAME_Y, {
    font: NAME_FONT,
    tracking: NAME_TRACK,
    fill: PALETTE.paper,
    glyph: (_g, i) => {
      const p = progress(start(i), start(i) + dur, lt);
      if (p <= 0) return null;
      return { dy: rise * (1 - landing(p)), alpha: clamp(p / 0.5) };
    },
  });
}

/** 0 to 1 with a small overshoot past 1 at about 70% and exactly 1 at the end (back-out). */
function landing(p: number): number {
  if (p >= 1) return 1;
  const s = 1.2;
  return 1 + (s + 1) * (p - 1) ** 3 + s * (p - 1) ** 2;
}

/**
 * How far the h's ink sits right of its origin at 220 px, less the G's at
 * the tagline's size, so the name's stem lines up with the G below it.
 * Measured each frame (two glyphs), so a frame drawn before the fonts load
 * cannot fix a wrong value for the rest of the run.
 */
function nameBearing(ctx: CanvasRenderingContext2D): number {
  ctx.save();
  ctx.font = NAME_FONT;
  const h = -ctx.measureText("h").actualBoundingBoxLeft;
  ctx.font = taglineStyle(ctx).font;
  const g = -ctx.measureText("G").actualBoundingBoxLeft;
  ctx.restore();
  return Math.max(0, h - g);
}

/** 72 px, or 64 px if 72 would run past 1360 px (type.ts's layout cache, reset when the fonts load). */
function taglineStyle(ctx: CanvasRenderingContext2D): WordStyle {
  const big = wordStyle(72);
  return layout(ctx, TAGLINE, big.font).width > TAGLINE_MAX ? wordStyle(64) : big;
}

/** The rounded box's outline length. */
const PERIMETER = 2 * (BOX.w + BOX.h) - 8 * BOX.r + 2 * Math.PI * BOX.r;

/**
 * The install box: its outline draws on from the top-left corner, bright,
 * then the fill comes up and the edge settles to the hairline (card.ts's
 * reveal); the prompt, the typed command and a terminal cursor inside.
 */
function drawInstall(ctx: CanvasRenderingContext2D, lt: number, t: number): void {
  const reveal = progress(BOX_IN[0], BOX_IN[1], lt);
  if (reveal <= 0) return;
  const body = smoothstep(0.35, 1, reveal);
  ctx.save();
  roundedRect(ctx, BOX.x, BOX.y, BOX.w, BOX.h, BOX.r);
  if (body > 0) {
    ctx.save();
    ctx.globalAlpha *= body;
    ctx.fillStyle = PALETTE.surface;
    ctx.fill();
    ctx.restore();
  }
  if (reveal < 1) {
    // Drawn over the box's first half (about eight frames at 60 fps, eased
    // in and out so it visibly travels), closing as the `$` lands on b3.75.
    ctx.setLineDash([PERIMETER * inOutSine(progress(0, 0.5, reveal)), PERIMETER + 20]);
    ctx.strokeStyle = mix(PALETTE.text2, PALETTE.divider, body);
    ctx.lineWidth = 2 - body;
  } else {
    ctx.strokeStyle = PALETTE.divider;
    ctx.lineWidth = 1;
  }
  ctx.stroke();
  ctx.restore();

  if (lt < T_PROMPT) return;
  const adv = 0.6 * MONO_SIZE;
  const cell = (col: number) => PROMPT_X + col * adv;
  ctx.save();
  ctx.font = font(MONO_SIZE, 400, MONO);
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
  // Each key lands a touch large and bright and settles in a 32nd.
  const key = (ch: string, col: number, at: number, color: string, bright: string) => {
    const d = lt - at;
    if (d < 0 || ch === " ") return;
    // On screen the frame it is typed, as a terminal shows it, so the
    // cursor never runs ahead of an unseen key; the size and heat settle.
    const k = swiftOut(progress(0, KEY_EVERY * 1.5, d));
    const s = lerp(1.22, 1, k);
    ctx.save();
    ctx.translate(cell(col) + adv / 2, INSTALL_Y);
    ctx.scale(s, s);
    ctx.fillStyle = mix(bright, color, k);
    ctx.fillText(ch, -adv / 2, 0);
    ctx.restore();
  };
  key("$", 0, T_PROMPT, PALETTE.cyan, PALETTE.glint);
  let typed = 0;
  Array.from(INSTALL).forEach((ch, i) => {
    const at = T_KEY0 + i * KEY_EVERY;
    if (lt >= at) typed = i + 1;
    key(ch, PROMPT.length + i, at, PALETTE.warm, PALETTE.warmBright);
  });
  ctx.restore();

  // The cursor: solid from the prompt until a beat after the last key (a
  // terminal holds it on while you type), then blinking on global beats,
  // out from b9.
  if (lt < CURSOR_OFF && (lt < b(6) || cursorOn(t))) {
    const col = PROMPT.length + typed;
    const h = 72;
    const top = BOX.y + (BOX.h - h) / 2;
    ctx.save();
    ctx.fillStyle = TERM.cursor;
    ctx.fillRect(cell(col) + 2, top, adv - 4, h);
    ctx.restore();
  }
}

function drawCopy(ctx: CanvasRenderingContext2D, lt: number, t: number): void {
  drawName(ctx, lt);
  drawWords(ctx, TAGLINE, LEFT, TAGLINE_Y, taglineStyle(ctx), lt, T_TAGLINE);
  drawInstall(ctx, lt, t);
  // The address lands bright on the bell and cools to cyan over a
  // sixteenth, as the typed keys do.
  const cool = smoothstep(T_URL, T_URL + b(0.25), lt);
  drawWords(ctx, URL_TEXT, LEFT, URL_Y, { ...URL_STYLE, fill: mix(PALETTE.glint, PALETTE.cyan, cool) }, lt, T_URL);
}

/**
 * The card, as a lit screen: the vignette gives way over the mark and the
 * copy as the bloom comes up, to a quarter of its strength there, so the
 * wordmark keeps one cyan end to end and the paper type stays paper. The
 * frame's corners still fall off.
 */
const CARD_LIT = { x: 160, y: 137, w: 1484, h: 756 } as const;
const CARD_LIT_ALPHA = 0.75;

export const scene: Scene = {
  id: S.id,
  start: S.start,
  end: S.end,
  draw(ctx, lt, env) {
    // The inherited frame, exactly: the wordmark minus its barb, on night.
    if (lt <= 0) {
      drawHandoff(ctx, "morph|end", env);
      return;
    }
    // From b10 the card is still: every frame is b10's (the cursor is already out).
    const at = Math.min(lt, STILL);
    ctx.fillStyle = PALETTE.night;
    ctx.fillRect(0, 0, env.W, env.H);
    drawMark(ctx, at);
    drawCopy(ctx, at, env.t);
  },
  // Null on the first frame, as morph|end is; up with the bloom over b0–2.
  lit: (lt) => {
    const a = CARD_LIT_ALPHA * smoothstep(0, b(2), Math.min(lt, STILL));
    return a > 0 ? { ...CARD_LIT, alpha: a } : null;
  },
  captions: () => CAPTIONS,
};

