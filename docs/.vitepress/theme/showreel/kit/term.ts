// K2: hk's terminal. A window (chrome bar, three dots, a hairline edge) and
// a fixed character grid: every character sits at x0 + col × advance, with
// the advance 0.6 of the size (Liberation Mono's own), and nothing is
// measured, so the glyphs Liberation Mono lacks (✔ ✗ ❯ ⚠ and the braille
// spinner) are drawn as vectors in their cells and can never shift a column.
//
// Lines carry no markup. Each line's colours are derived from its text by
// the rules hk's own output follows (src/hook.rs, src/step/progress.rs, clx),
// in Catppuccin Frappé as the README demo shows them; screens.test.ts checks
// the rules against the ANSI the captured runs printed. The spinner and the
// cursor run on GLOBAL time, so a pane that crosses a bar line keeps both in
// phase.

import { type LitRect, TERM } from "../bible";
import { rgba } from "../color";
import { roundedRect } from "../fx";
import { lerp, TAU } from "../math";
import { font, MONO } from "../type";
import { cursorOn, type Rect } from "./motion";
import { VERSION } from "./screens";

/** The chrome bar's height, px. */
export const CHROME = 44;
/** The window's corner radius, px. */
export const RADIUS = 12;

/** A character cell's width at `size` px: Liberation Mono's advance, 0.6 em. */
export const advance = (size: number): number => 0.6 * size;

/** Where a terminal stands and how its text is set. */
export interface Pane extends Rect {
  /** A chrome bar with three dots across the top. */
  chrome: boolean;
  /** Paint the window. MINI's is off: it sits in a panel the scene draws. */
  window: boolean;
  /** Mono px, and baseline to baseline. */
  size: number;
  lineH: number;
  /** Column 0's left edge, and row 0's baseline. */
  x0: number;
  baseline0: number;
  /** Rows the pane holds. */
  rows: number;
  /** Width of the fade that cuts lines off at the window's right edge, px; 0 where 80 columns fit. */
  fade: number;
  /**
   * The rows a screen taller than the pane shows: the bottom ones, as a
   * terminal scrolls, or the top ones (MINI, and race's F0 pane as
   * `{ ...PANE_FULL, anchor: "top" }`).
   */
  anchor: "bottom" | "top";
}

/** The full terminal: 80 columns and 11 rows of 32 px mono, baselines 234 to 674. */
export const PANE_FULL: Pane = {
  x: 160,
  y: 140,
  w: 1600,
  h: 560,
  chrome: true,
  window: true,
  size: 32,
  lineH: 44,
  x0: 192,
  baseline0: 234,
  rows: 11,
  fade: 0,
  anchor: "bottom",
};

/** The terminal cut down to its last two rows above a scene: baselines 190 and 234. */
export const STRIP: Pane = { ...PANE_FULL, y: 100, h: 164, baseline0: 190, rows: 2 };

/** A small terminal beside the action (catch): 8 rows of 26 px, cut off by a 60 px fade. */
export const INSET: Pane = {
  x: 120,
  y: 110,
  w: 760,
  h: 320,
  chrome: false,
  window: true,
  size: 26,
  lineH: 36,
  x0: 140,
  baseline0: 147,
  rows: 8,
  fade: 60,
  anchor: "bottom",
};

/**
 * The terminal inside one of everywhere's panels (x, w): the top 9 rows of
 * a screen in 24 px, baselines 417 + 32·i, faded 48 px before the panel's
 * right edge. The panel is the scene's; this draws only the text.
 */
export function mini(x: number, w = 520): Pane {
  return { x, y: 384, w, h: 306, chrome: false, window: false, size: 24, lineH: 32, x0: x + 18, baseline0: 417, rows: 9, fade: 48, anchor: "top" };
}

/**
 * A pane `k` of the way from `a` to `b` (stash squeezes PANE_FULL into
 * STRIP). The numbers tween; the switches flip halfway. Pass a scroll to
 * drawTerm as well, so lines leave through the top as the pane shrinks.
 */
export function lerpPane(a: Pane, b: Pane, k: number): Pane {
  const n = (key: "x" | "y" | "w" | "h" | "size" | "lineH" | "x0" | "baseline0" | "rows" | "fade") => lerp(a[key], b[key], k);
  const s = k < 0.5 ? a : b;
  return {
    x: n("x"),
    y: n("y"),
    w: n("w"),
    h: n("h"),
    chrome: s.chrome,
    window: s.window,
    size: n("size"),
    lineH: n("lineH"),
    x0: n("x0"),
    baseline0: n("baseline0"),
    rows: n("rows"),
    fade: n("fade"),
    anchor: s.anchor,
  };
}

/** The window's body: under the chrome bar, if there is one. */
export const bodyOf = (p: Pane): Rect => {
  const top = p.chrome ? Math.min(CHROME, p.h) : 0;
  return { x: p.x, y: p.y + top, w: p.w, h: p.h - top };
};

/**
 * The first of `n` lines a pane shows: 0 while they fit, then the bottom
 * rows (a terminal scrolls), or always 0 for a top-anchored pane.
 */
export function firstLine(p: Pane, n: number): number {
  return p.anchor === "top" ? 0 : Math.max(0, n - Math.floor(p.rows + 1e-9));
}

/** Where a screen's lines and cells are in a pane. */
export interface TermLayout {
  /** The first line shown; fractional while a scene scrolls. */
  first: number;
  /** Column width. */
  advance: number;
  /** Line i's baseline (i indexes the screen's lines, not the pane's rows). */
  baseline(i: number): number;
  /** Column c's left edge. */
  col(c: number): number;
  /**
   * Line i's column c as a terminal cell: a line's height, the text's em box
   * centred in it. The cursor fills it; a glyph's centre is its middle.
   */
  cell(i: number, c: number): Rect;
  /** The window's body, where the text is clipped. */
  body: Rect;
}

/** The layout of an `n`-line screen in `p`, scrolled to `scroll` (default firstLine). */
export function termLayout(p: Pane, n: number, scroll?: number): TermLayout {
  const first = scroll ?? firstLine(p, n);
  const adv = advance(p.size);
  const baseline = (i: number) => p.baseline0 + (i - first) * p.lineH;
  const col = (c: number) => p.x0 + c * adv;
  return {
    first,
    advance: adv,
    baseline,
    col,
    cell: (i, c) => ({ x: col(c), y: baseline(i) - 0.8 * p.size - (p.lineH - p.size) / 2, w: adv, h: p.lineH }),
    body: bodyOf(p),
  };
}

// hk's header, character for character.

/**
 * The header hk draws above a run (src/hook.rs start_hk_progress, clx's
 * flex progress bar), `cols` wide: `hk 2.2.0 by @jdx – pre-commit – fix`,
 * then a bar that fills the rest of the line and `cur/total`. The bar is
 * empty at 0, full at `total`, and otherwise `round(cells·cur/total)` cells
 * with `>` at the head, as clx rounds it.
 */
export function header(hook: "pre-commit" | null, run: "fix" | "check", cur: number, total: number, cols = 80): string {
  const prefix = `hk ${VERSION} by @jdx${hook ? ` – ${hook}` : ""} – ${run}  `;
  const count = `${cur}/${total}`;
  // The line's width less the brackets and the space before the count.
  const cells = Math.max(0, cols - prefix.length - 3 - count.length);
  const progress = total > 0 ? cur / total : 0;
  const n = Math.round(cells * progress);
  const bar = progress >= 1 ? "=".repeat(cells) : n > 0 ? `${"=".repeat(n - 1)}>${" ".repeat(cells - n)}` : " ".repeat(cells);
  return `${prefix}[${bar}] ${count}`;
}

// Colours, from the text.

/** A stretch of a line in one colour and weight. */
export interface Run {
  text: string;
  color: string;
  bold: boolean;
}

const run = (text: string, color: string = TERM.text, bold = false): Run => ({ text, color, bold });

/** hk's header: `hk` bold magenta, everything to the bar's end dim, then the count. */
const HEADER = /^(hk)( \S+ by @jdx(?: – \S+)* – (?:fix|check) {2}\[[=> ]*\] )(\d+\/\d+)$/;
/** A step's state glyph, then its name and message. */
const STEP = /^([✔✗⚠❯])( .*)$/u;
/** A running command under its step (or the files step itself): the spinner, blue. */
const SPIN = /^( ?)([⠀-⣿])(.*)$/u;
/** A failed step's tail: `✗ shellcheck  – ERROR`, `⚠ prettier  – aborted`. */
const TAIL = /^(.* – )(ERROR|aborted)$/;
/** hk's fix hint in a failure's footer. */
const FIX_HINT = /^(hk) (ERROR)( To fix, run: )(.+)$/;

const GLYPH_COLOR: Record<string, string> = { "✔": TERM.green, "✗": TERM.red, "⚠": TERM.yellow, "❯": TERM.dim };

/**
 * A line's colours, from its text alone, as hk printed it:
 * - the header: `hk` bold magenta, the version, `by @jdx`, ` – <hook>`,
 *   ` – fix`/` – check` and the bar dim, `cur/total` default;
 * - `✔` green, `✗` and a failed step's `ERROR` red, `⚠` and `aborted`
 *   yellow, `❯` dim, each followed by the step in the default colour;
 * - the spinner blue, before a running command or the files step;
 * - the shell's `$ ` in the prompt colour;
 * - `<step> stderr:` bold, and a fix hint's `hk ERROR` red with its
 *   command dim;
 * - everything else, tool and git output included, in the default colour.
 */
export function styleLine(text: string): Run[] {
  let m = HEADER.exec(text);
  if (m) return [run(m[1], TERM.magenta, true), run(m[2], TERM.dim), run(m[3])];
  m = STEP.exec(text);
  if (m) {
    const glyph = run(m[1], GLYPH_COLOR[m[1]]);
    const tail = m[1] === "✗" || m[1] === "⚠" ? TAIL.exec(m[2]) : null;
    return tail ? [glyph, run(tail[1]), run(tail[2], m[1] === "✗" ? TERM.red : TERM.yellow)] : [glyph, run(m[2])];
  }
  m = SPIN.exec(text);
  if (m) return [run(m[1]), run(m[2], TERM.blue), run(m[3])].filter((r) => r.text);
  if (text.startsWith("$ ")) return [run("$ ", TERM.prompt), run(text.slice(2))];
  if (/^\S+ stderr:$/.test(text)) return [run(text, TERM.text, true)];
  m = FIX_HINT.exec(text);
  if (m) return [run(m[1], TERM.red), run(" "), run(m[2], TERM.red), run(m[3]), run(m[4], TERM.dim)];
  return [run(text)];
}

// The glyphs Liberation Mono lacks, drawn in their cells.

/** clx's mini_dot spinner, one frame per 200 ms. */
export const SPINNER = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏";

/** The spinner's frame at GLOBAL time `t`. */
export const spinnerFrame = (t: number): string => SPINNER[Math.floor(t / 0.2 + 1e-9) % SPINNER.length];

/** Characters term.ts draws itself: the step glyphs and all of braille. */
export const isVectorGlyph = (ch: string): boolean => ch in GLYPH_COLOR || (ch >= "⠀" && ch <= "⣿");

/**
 * Draw a vector glyph in the cell whose left edge is `x`, on baseline `y`,
 * at `size` px, in `paint`. Each is sized to Liberation Mono's cap height
 * (0.66 em) and weight, so it reads as the font's own.
 */
function drawGlyph(ctx: CanvasRenderingContext2D, ch: string, x: number, y: number, size: number, paint: string | CanvasGradient): void {
  const w = advance(size);
  const u = size;
  ctx.save();
  ctx.strokeStyle = paint;
  ctx.fillStyle = paint;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  if (ch === "✔") {
    // Heavy check mark: a short down-stroke into a long rising one, a
    // little taller than the x-height, as the README demo's terminal draws it.
    ctx.lineWidth = 0.14 * u;
    ctx.moveTo(x + 0.12 * w, y - 0.3 * u);
    ctx.lineTo(x + 0.4 * w, y - 0.06 * u);
    ctx.lineTo(x + 0.9 * w, y - 0.6 * u);
    ctx.stroke();
  } else if (ch === "✗") {
    // Ballot X: two strokes, the rising one a little bowed, as it is drawn by hand.
    ctx.lineWidth = 0.11 * u;
    ctx.moveTo(x + 0.14 * w, y - 0.62 * u);
    ctx.lineTo(x + 0.86 * w, y - 0.04 * u);
    ctx.moveTo(x + 0.86 * w, y - 0.64 * u);
    ctx.quadraticCurveTo(x + 0.42 * w, y - 0.3 * u, x + 0.12 * w, y - 0.04 * u);
    ctx.stroke();
  } else if (ch === "❯") {
    // Heavy right-pointing angle: a chevron at the cell's middle.
    ctx.lineWidth = 0.13 * u;
    ctx.moveTo(x + 0.24 * w, y - 0.6 * u);
    ctx.lineTo(x + 0.8 * w, y - 0.33 * u);
    ctx.lineTo(x + 0.24 * w, y - 0.06 * u);
    ctx.stroke();
  } else if (ch === "⚠") {
    // Warning sign: a triangle round an exclamation mark.
    ctx.lineWidth = 0.075 * u;
    ctx.moveTo(x + 0.5 * w, y - 0.72 * u);
    ctx.lineTo(x + 0.97 * w, y - 0.02 * u);
    ctx.lineTo(x + 0.03 * w, y - 0.02 * u);
    ctx.closePath();
    ctx.stroke();
    ctx.beginPath();
    ctx.lineWidth = 0.08 * u;
    ctx.moveTo(x + 0.5 * w, y - 0.46 * u);
    ctx.lineTo(x + 0.5 * w, y - 0.26 * u);
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(x + 0.5 * w, y - 0.13 * u, 0.05 * u, 0, TAU);
    ctx.fill();
  } else {
    // Braille: a 2×4 matrix of dots, r 0.09 em; bit k of the code point
    // after U+2800 is dot k + 1 (1–3 down the left, 4–6 down the right,
    // then 7 and 8 in the descender). Dots 1–6, all the spinner uses, span
    // cap height to baseline, as a font's braille does.
    const bits = ch.codePointAt(0)! - 0x2800;
    const r = 0.09 * u;
    const cx = [x + 0.5 * w - 0.13 * u, x + 0.5 * w + 0.13 * u];
    const cy = (row: number) => y - 0.57 * u + 0.24 * u * row;
    const DOTS: readonly [col: number, row: number][] = [
      [0, 0],
      [0, 1],
      [0, 2],
      [1, 0],
      [1, 1],
      [1, 2],
      [0, 3],
      [1, 3],
    ];
    DOTS.forEach(([c, row], k) => {
      if (!(bits & (1 << k))) return;
      ctx.moveTo(cx[c] + r, cy(row));
      ctx.arc(cx[c], cy(row), r, 0, TAU);
    });
    ctx.fill();
  }
  ctx.restore();
}

// Drawing.

export interface LineOptions {
  /** GLOBAL seconds (env.t): the spinner's frame. Without it a spinner keeps the frame in the text. */
  t?: number;
  /** Fade the text out from x0 to x1 (a pane's right edge), px. */
  fade?: readonly [x0: number, x1: number] | null;
}

/**
 * One terminal line on the grid: column 0's left edge at `x0`, on baseline
 * `y`, at `size` px, coloured by styleLine. For a line a scene moves by
 * itself; drawTerm draws a pane's lines with it.
 */
export function drawTermLine(ctx: CanvasRenderingContext2D, text: string, x0: number, y: number, size: number, o: LineOptions = {}): void {
  const adv = advance(size);
  // A fade is each run's colour turning transparent across the fade, so it
  // stays exact under any alpha the caller has set.
  const paint = (color: string): string | CanvasGradient => {
    if (!o.fade) return color;
    const g = ctx.createLinearGradient(o.fade[0], 0, o.fade[1], 0);
    g.addColorStop(0, color);
    g.addColorStop(1, rgba(color, 0));
    return g;
  };
  ctx.save();
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
  let col = 0;
  for (const r of styleLine(text)) {
    const fill = paint(r.color);
    ctx.font = font(size, r.bold ? 700 : 400, MONO);
    ctx.fillStyle = fill;
    // One character per column, never a run set whole: Chromium rounds a
    // run's advances to whole pixels (19 px, not 19.2, at 32 px), which
    // walks a long run off its columns (`]` of the header lands 11 px left
    // of its cell and opens a double space before the count).
    for (const ch of r.text) {
      const x = x0 + col * adv;
      if (isVectorGlyph(ch)) drawGlyph(ctx, o.t !== undefined && SPINNER.includes(ch) ? spinnerFrame(o.t) : ch, x, y, size, fill);
      else if (ch !== " ") ctx.fillText(ch, x, y);
      col++;
    }
  }
  ctx.restore();
}

/**
 * Where drawTermLine puts character `col` of `text` (x0, size as there):
 * its cell's left edge, on the grid, and the width Chromium gives the
 * character drawn alone, px. A glyph drawn centred on x + w / 2 therefore
 * lands exactly on the one drawTermLine draws: for a scene that moves a
 * glyph onto a line the terminal then draws itself.
 */
export function termCharX(ctx: CanvasRenderingContext2D, text: string, col: number, x0: number, size: number): { x: number; w: number } {
  const adv = advance(size);
  const x = x0 + col * adv;
  let c = 0;
  for (const r of styleLine(text)) {
    for (const ch of r.text) {
      if (c === col) {
        if (isVectorGlyph(ch)) return { x, w: adv };
        ctx.save();
        ctx.font = font(size, r.bold ? 700 : 400, MONO);
        const w = ctx.measureText(ch).width;
        ctx.restore();
        return { x, w };
      }
      c++;
    }
  }
  return { x, w: adv };
}

/**
 * A terminal window at `r`: the body, the chrome bar with its three dots
 * if `chrome`, and the 1 px edge, drawn inside the rect so a lit rect of
 * it covers it exactly.
 */
export function drawWindow(ctx: CanvasRenderingContext2D, r: Rect, chrome: boolean): void {
  if (r.w <= 0 || r.h <= 0) return;
  ctx.save();
  roundedRect(ctx, r.x, r.y, r.w, r.h, RADIUS);
  ctx.fillStyle = TERM.window;
  ctx.fill();
  if (chrome) {
    ctx.save();
    ctx.clip();
    ctx.fillStyle = TERM.chrome;
    ctx.fillRect(r.x, r.y, r.w, Math.min(CHROME, r.h));
    TERM.dots.forEach((c, i) => {
      ctx.fillStyle = c;
      ctx.beginPath();
      ctx.arc(r.x + 28 + 24 * i, r.y + CHROME / 2, 7, 0, TAU);
      ctx.fill();
    });
    ctx.restore();
  }
  roundedRect(ctx, r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1, RADIUS - 0.5);
  ctx.strokeStyle = TERM.edge;
  ctx.lineWidth = 1;
  ctx.stroke();
  ctx.restore();
}

export interface TermOptions {
  /** GLOBAL seconds (env.t), never local time: the spinner's frame and the cursor's blink. */
  t: number;
  /** The first line shown, fractional while a scene scrolls (default firstLine). */
  scroll?: number;
  /** 0..1 the whole pane. */
  alpha?: number;
  /** 0..1 line i (an index into `lines`): a row fading in or out. */
  lineAlpha?: (i: number) => number;
  /**
   * A rosewater block cursor, on for a beat and off for the next on global
   * beats: after the last line's text, or at a line and column. hk hides
   * the cursor while it runs, so only a prompt waiting for input has one.
   */
  cursor?: boolean | { line: number; col: number };
}

/**
 * A terminal: the window (unless the pane's is off) and a screen's lines
 * on its grid, the bottom rows of a screen longer than the pane (or the top
 * rows, for a top-anchored pane), clipped to the body and faded at the
 * right edge where 80 columns do not fit. Returns where everything is.
 */
export function drawTerm(ctx: CanvasRenderingContext2D, p: Pane, lines: readonly string[], o: TermOptions): TermLayout {
  const L = termLayout(p, lines.length, o.scroll);
  const a = o.alpha ?? 1;
  if (a <= 0) return L;
  ctx.save();
  ctx.globalAlpha *= a;
  if (p.window) drawWindow(ctx, p, p.chrome);
  const { body } = L;
  if (body.w <= 0 || body.h <= 0) {
    ctx.restore();
    return L;
  }
  roundedRect(ctx, p.x, p.y, p.w, p.h, RADIUS);
  ctx.clip();
  ctx.beginPath();
  ctx.rect(body.x, body.y, body.w, body.h);
  ctx.clip();
  const fade = p.fade > 0 ? ([p.x + p.w - p.fade, p.x + p.w] as const) : null;
  lines.forEach((text, i) => {
    const row = i - L.first;
    if (row <= -1 || row >= p.rows) return;
    const la = o.lineAlpha ? o.lineAlpha(i) : 1;
    if (la <= 0) return;
    ctx.save();
    ctx.globalAlpha *= Math.min(1, la);
    drawTermLine(ctx, text, p.x0, L.baseline(i), p.size, { t: o.t, fade });
    ctx.restore();
  });
  if (o.cursor && cursorOn(o.t)) {
    const at = o.cursor === true ? { line: lines.length - 1, col: Array.from(lines.at(-1) ?? "").length } : o.cursor;
    const c = L.cell(at.line, at.col);
    if (fade) {
      const g = ctx.createLinearGradient(fade[0], 0, fade[1], 0);
      g.addColorStop(0, TERM.cursor);
      g.addColorStop(1, rgba(TERM.cursor, 0));
      ctx.fillStyle = g;
    } else {
      ctx.fillStyle = TERM.cursor;
    }
    ctx.fillRect(c.x, c.y, c.w, c.h);
  }
  ctx.restore();
  return L;
}

/** A terminal's window as the lit screen the vignette spares (Scene.lit). */
export const termLit = (r: Rect, alpha = 1): LitRect => ({ x: r.x, y: r.y, w: r.w, h: r.h, alpha });
