// K4: code cards (storyboard §4 K4), used by config, commit, stash, restore,
// catch and everywhere. A card is a file shown as an editor would: a
// surface panel with the file's name on a tab and its lines in mono on the
// 0.6 em grid, hk.pkl in Pkl colours and everything else plain. Scenes lay
// their own marks over the lines (squiggles, bands, strike-throughs) with
// the layout drawCard returns.
//
// The file contents are verbatim: HKPKL_LINES is the capture repo's
// hk.pkl steps block, MAIN_PY_STAGED what was staged of src/main.py (hk's
// stash backup patch, without the unstaged last line), and MAIN_PY_FIXED
// `git show ada2ca4:src/main.py`, the version the commit got.

import { PALETTE, TERM } from "../bible";
import { mix, rgba } from "../color";
import { roundedRect } from "../fx";
import { clamp, progress } from "../math";
import { drawText, font, MONO } from "../type";

export interface CardRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/**
 * A line of a card: text, or a folded pill standing for lines the card
 * does not show. `color` sets the whole line in one colour (the stash's
 * unstaged line in paper); `chars` types it on.
 */
export type CardLine =
  | string
  | {
      text?: string;
      pill?: string;
      color?: string;
      alpha?: number;
      chars?: number;
      /** Nudge, px. */
      dx?: number;
      dy?: number;
    };

export interface CardOptions {
  /** The file's name on the tab. */
  tab?: string;
  lines: readonly CardLine[];
  /** Mono size and line pitch, px. */
  size: number;
  lineH: number;
  /** Pkl colouring for hk.pkl; plain text1 otherwise. */
  lang?: "pkl" | "plain";
  /** Column 0's x: the card's left + 24 by default (the config scene sets 600). */
  textX?: number;
  /** A strip down the left edge beside rows `from`..`to`: the stash card's 6 px cyanSoft staged gutter. */
  gutter?: { color: string; from?: number; to?: number; width?: number };
  /** A band behind rows `from`..`to`, drawn on left to right by `sweep` 0..1. */
  highlight?: { from: number; to: number; color?: string; sweep?: number; alpha?: number };
  alpha?: number;
  /** The config scene's stroke-draw, 0..1: the outline draws on, then the fill and tab come up. */
  reveal?: number;
  /**
   * Draw the card's box (surface fill and divider edge). False draws only
   * what is inside it, clipped to it, for a scene that draws the box itself
   * (the config card shrinking into the hk.pkl chip). Default true.
   */
  frame?: boolean;
  /** The tab's label's opacity, 0..1 (the tab itself stays): a scene flying the name elsewhere. Default 1. */
  tabLabelAlpha?: number;
}

/** Where a card's rows are, for scenes laying marks over them. */
export interface CardLayout {
  rect: CardRect;
  /** Column 0's x. */
  x0: number;
  /** One column, px: 0.6 of the size. */
  advance: number;
  baseline(row: number): number;
  /** A row's box: lineH tall around its text. */
  rowTop(row: number): number;
  rowBottom(row: number): number;
  /** Column `col`'s left edge. */
  col(col: number): number;
}

/** The tab strip across a card's top, px. */
const TAB_H = 40;

/**
 * The tab strip's height and its label's size and inset, px: the label's
 * baseline is at the card's y + h / 2 + 0.3 · size, its left at x + pad.
 */
export const CARD_TAB = { h: TAB_H, size: 30, pad: 24 } as const;

/** A card's rows: the first baseline is y + 40 + 12 + 0.8·size. */
export function cardLayout(rect: CardRect, o: { size: number; lineH: number; textX?: number }): CardLayout {
  const x0 = o.textX ?? rect.x + 24;
  const advance = 0.6 * o.size;
  const baseline = (row: number) => rect.y + TAB_H + 12 + 0.8 * o.size + row * o.lineH;
  const rowTop = (row: number) => baseline(row) - 0.8 * o.size - (o.lineH - o.size) / 2;
  return {
    rect,
    x0,
    advance,
    baseline,
    rowTop,
    rowBottom: (row) => rowTop(row) + o.lineH,
    col: (c) => x0 + c * advance,
  };
}

/** The card's box alone: surface, a 1 px divider edge, radius 16. The catch scene's commit card is one. */
export function drawPanel(
  ctx: CanvasRenderingContext2D,
  r: CardRect,
  o: { radius?: number; fill?: string; stroke?: string; lineWidth?: number; alpha?: number } = {},
): void {
  const a = o.alpha ?? 1;
  if (a <= 0 || r.w <= 0 || r.h <= 0) return;
  ctx.save();
  ctx.globalAlpha *= a;
  roundedRect(ctx, r.x, r.y, r.w, r.h, o.radius ?? 16);
  ctx.fillStyle = o.fill ?? PALETTE.surface;
  ctx.fill();
  ctx.strokeStyle = o.stroke ?? PALETTE.divider;
  ctx.lineWidth = o.lineWidth ?? 1;
  ctx.stroke();
  ctx.restore();
}

/** One run of a line: its text and colour, placed by column. */
export interface Run {
  text: string;
  color: string;
}

const PKL_KEYWORDS = new Set(["amends", "import", "new", "local", "depends"]);

/**
 * Pkl colouring: strings cyan; amends, import, new, local and depends
 * warm; `Builtins.x` text1; braces, brackets, parentheses and `=` text2;
 * everything else text1.
 */
export function pklRuns(line: string): Run[] {
  const runs: Run[] = [];
  const push = (text: string, color: string) => {
    const last = runs[runs.length - 1];
    if (last && last.color === color) last.text += text;
    else runs.push({ text, color });
  };
  const token = /"[^"]*"?|[A-Za-z_][\w.]*|[{}[\]()=]|\s+|./g;
  for (const [t] of line.matchAll(token)) {
    if (t.startsWith('"')) push(t, PALETTE.cyan);
    else if (PKL_KEYWORDS.has(t)) push(t, PALETTE.warm);
    else if (/^[{}[\]()=]$/.test(t)) push(t, PALETTE.text2);
    else push(t, PALETTE.text1);
  }
  return runs;
}

/**
 * Mono text on the 0.6 em grid: each character drawn in its own column,
 * never measured. Chromium rounds a string's advances to whole pixels at
 * some sizes (20 px, not 20.4, at 34 px), which would walk a long line off
 * its columns and off the lines above and below it.
 */
export function drawMono(ctx: CanvasRenderingContext2D, text: string, x: number, y: number, size: number, color: string, weight = 400): void {
  ctx.save();
  ctx.font = font(size, weight, MONO);
  ctx.textAlign = "left";
  ctx.textBaseline = "alphabetic";
  ctx.fillStyle = color;
  let col = 0;
  for (const ch of text) {
    if (ch !== " ") ctx.fillText(ch, x + col * 0.6 * size, y);
    col++;
  }
  ctx.restore();
}

/** Width of `text` in mono on the grid, px. */
export const monoWidth = (text: string, size: number): number => Array.from(text).length * 0.6 * size;

/** Draw runs from column 0 at x0, typing on the first `chars` characters. */
function drawRuns(ctx: CanvasRenderingContext2D, runs: readonly Run[], x0: number, y: number, size: number, chars = Infinity): void {
  let col = 0;
  for (const r of runs) {
    if (col >= chars) break;
    const glyphs = Array.from(r.text);
    drawMono(ctx, glyphs.slice(0, Math.max(0, Math.floor(chars - col))).join(""), x0 + col * 0.6 * size, y, size, r.color);
    col += glyphs.length;
  }
}

/** The folded pill: mono 28 px text3 on an elevated pill, centred on the row. */
function drawPill(ctx: CanvasRenderingContext2D, text: string, x: number, midY: number, lineH: number): void {
  const size = 28;
  const w = monoWidth(text, size) + 32;
  const h = Math.min(lineH - 6, 40);
  ctx.save();
  roundedRect(ctx, x, midY - h / 2, w, h, h / 2);
  ctx.fillStyle = PALETTE.elevated;
  ctx.fill();
  ctx.restore();
  drawMono(ctx, text, x + 16, midY + 0.3 * size, size, PALETTE.text3);
}

/** Length of a rounded rect's outline. */
const perimeter = (r: CardRect, radius: number) => 2 * (r.w + r.h) - 8 * radius + 2 * Math.PI * radius;

/**
 * A code card: surface fill, 1 px divider edge, radius 16; a 40 px tab
 * strip with the file's name (mono 30 px text2) on a surface tab at the
 * top left; lines from x + 24, the first baseline at y + 40 + 12 + 0.8·size.
 * The text is clipped to the card, so a card can shrink round it. Returns
 * the layout for overlays.
 */
export function drawCard(ctx: CanvasRenderingContext2D, rect: CardRect, o: CardOptions): CardLayout {
  const L = cardLayout(rect, o);
  const a = o.alpha ?? 1;
  if (a <= 0 || rect.w <= 0 || rect.h <= 0) return L;
  const reveal = clamp(o.reveal ?? 1);
  const body = progress(0.45, 1, reveal);
  const R = 16;
  ctx.save();
  ctx.globalAlpha *= a;

  roundedRect(ctx, rect.x, rect.y, rect.w, rect.h, R);
  if (body > 0 && o.frame !== false) {
    ctx.save();
    ctx.globalAlpha *= body;
    ctx.fillStyle = PALETTE.surface;
    ctx.fill();
    ctx.restore();
  }
  if (o.frame !== false) {
    ctx.save();
    if (reveal < 1) {
      // Drawn on bright enough to see, settling to the hairline as the card fills.
      const len = perimeter(rect, R);
      ctx.setLineDash([len * progress(0, 0.6, reveal), len + 20]);
      ctx.strokeStyle = mix(PALETTE.text2, PALETTE.divider, body);
      ctx.lineWidth = 2 - body;
    } else {
      ctx.strokeStyle = PALETTE.divider;
      ctx.lineWidth = 1;
    }
    ctx.stroke();
    ctx.restore();
  }

  ctx.save();
  roundedRect(ctx, rect.x, rect.y, rect.w, rect.h, R);
  ctx.clip();
  if (o.tab && body > 0) {
    // An editor's tab strip, a step darker than the card, with the file's
    // tab in the card's own colour so it reads as open.
    ctx.save();
    ctx.globalAlpha *= body;
    const tw = Math.min(rect.w, monoWidth(o.tab, 30) + 48);
    ctx.fillStyle = TERM.window;
    ctx.fillRect(rect.x, rect.y, rect.w, TAB_H);
    ctx.fillStyle = PALETTE.surface;
    roundedRect(ctx, rect.x, rect.y, tw, TAB_H + 12, 10);
    ctx.fill();
    ctx.fillStyle = PALETTE.divider;
    ctx.fillRect(rect.x + tw, rect.y + TAB_H - 1, rect.w - tw, 1);
    const la = clamp(o.tabLabelAlpha ?? 1);
    if (la > 0) {
      ctx.globalAlpha *= la;
      drawMono(ctx, o.tab, rect.x + 24, rect.y + TAB_H / 2 + 0.3 * 30, 30, PALETTE.text2);
    }
    ctx.restore();
  }
  const g = o.gutter;
  if (g) {
    const from = g.from ?? 0;
    const to = g.to ?? o.lines.length - 1;
    ctx.fillStyle = g.color;
    ctx.fillRect(rect.x + 12, L.rowTop(from), g.width ?? 6, L.rowBottom(to) - L.rowTop(from));
  }
  const hl = o.highlight;
  if (hl && (hl.alpha ?? 1) > 0) {
    const sweep = clamp(hl.sweep ?? 1);
    const x = rect.x + 12;
    ctx.save();
    ctx.globalAlpha *= hl.alpha ?? 1;
    ctx.fillStyle = hl.color ?? "rgba(234,193,142,0.14)";
    ctx.fillRect(x, L.rowTop(hl.from), (rect.w - 24) * sweep, L.rowBottom(hl.to) - L.rowTop(hl.from));
    ctx.restore();
  }
  o.lines.forEach((line, i) => {
    const l = typeof line === "string" ? { text: line } : line;
    const la = l.alpha ?? 1;
    if (la <= 0) return;
    ctx.save();
    ctx.globalAlpha *= la;
    const x = L.x0 + (l.dx ?? 0);
    const y = L.baseline(i) + (l.dy ?? 0);
    if (l.pill !== undefined) drawPill(ctx, l.pill, x, y - 0.3 * o.size, o.lineH);
    if (l.text) {
      const runs = l.color ? [{ text: l.text, color: l.color }] : o.lang === "pkl" ? pklRuns(l.text) : [{ text: l.text, color: PALETTE.text1 }];
      drawRuns(ctx, runs, x, y, o.size, l.chars);
    }
    ctx.restore();
  });
  ctx.restore();
  ctx.restore();
  return L;
}

// hk.pkl.

/**
 * The capture repo's hk.pkl, steps block verbatim. Line 0 folds the real
 * `amends` and `import` lines (package URLs) into a pill.
 */
export const HKPKL_LINES: readonly CardLine[] = [
  { pill: "amends … import …" },
  "steps {",
  '  ["prettier"] = Builtins.prettier',
  '  ["ruff"] = Builtins.ruff',
  '  ["ruff-format"] = (Builtins.ruff_format) {',
  '    depends = "ruff"',
  "  }",
  '  ["shfmt"] = Builtins.shfmt',
  '  ["shellcheck"] = Builtins.shellcheck',
  '  ["trailing-whitespace"] = Builtins.trailing_whitespace',
  '  ["newlines"] = Builtins.newlines',
  "}",
];

/** The seven steps as the config scene plucks them: step name, builtin identifier, and its line in HKPKL_LINES. */
export const HKPKL_STEPS = [
  { step: "prettier", builtin: "prettier", line: 2 },
  { step: "ruff", builtin: "ruff", line: 3 },
  { step: "ruff-format", builtin: "ruff_format", line: 4 },
  { step: "shfmt", builtin: "shfmt", line: 7 },
  { step: "shellcheck", builtin: "shellcheck", line: 8 },
  { step: "trailing-whitespace", builtin: "trailing_whitespace", line: 9 },
  { step: "newlines", builtin: "newlines", line: 10 },
] as const;

/** The hk.pkl chip every scene hands on: x 850–1070, y 120–180. */
export const HKPKL_CHIP: CardRect & { r: number } = { x: 850, y: 120, w: 220, h: 60, r: 12 };

/**
 * The hk.pkl chip: surface, a 1.5 px cyan edge, radius 12, `hk.pkl` in mono
 * 36 px text1 centred on baseline 162. `rect` moves or resizes it (the label
 * scales with its height); `scale` grows it about its centre.
 */
export function drawChip(
  ctx: CanvasRenderingContext2D,
  o: { rect?: CardRect; scale?: number; alpha?: number; textAlpha?: number; stroke?: string } = {},
): void {
  const a = o.alpha ?? 1;
  if (a <= 0) return;
  const r = o.rect ?? HKPKL_CHIP;
  const k = r.h / HKPKL_CHIP.h;
  ctx.save();
  ctx.globalAlpha *= a;
  if (o.scale !== undefined && o.scale !== 1) {
    ctx.translate(r.x + r.w / 2, r.y + r.h / 2);
    ctx.scale(o.scale, o.scale);
    ctx.translate(-(r.x + r.w / 2), -(r.y + r.h / 2));
  }
  roundedRect(ctx, r.x, r.y, r.w, r.h, HKPKL_CHIP.r * k);
  ctx.fillStyle = PALETTE.surface;
  ctx.fill();
  ctx.strokeStyle = o.stroke ?? PALETTE.cyan;
  ctx.lineWidth = 1.5;
  ctx.stroke();
  const ta = o.textAlpha ?? 1;
  if (ta > 0) {
    drawText(ctx, "hk.pkl", r.x + r.w / 2, r.y + r.h / 2 + 12 * k, { font: font(36 * k, 400, MONO), fill: rgba(PALETTE.text1, ta), align: "center" });
  }
  ctx.restore();
}

// src/main.py.

/** What was staged of src/main.py: the file as the linters see it. */
export const MAIN_PY_STAGED: readonly string[] = [
  "import os",
  "def main()->None:",
  "    print( 'ahoy' )",
  "",
  "def hoist(sail:str)->str:",
  '    return f"hoisting {sail}"',
  "",
  "",
  'if __name__ == "__main__":',
  "    main()",
];

/** The unstaged 11th line, which hk stashes and puts back. */
export const MAIN_PY_TODO = "# TODO: splice the mainbrace";

/** src/main.py as committed, after ruff and ruff-format: `git show ada2ca4:src/main.py`. */
export const MAIN_PY_FIXED: readonly string[] = [
  "def main() -> None:",
  '    print("ahoy")',
  "",
  "",
  "def hoist(sail: str) -> str:",
  '    return f"hoisting {sail}"',
  "",
  "",
  'if __name__ == "__main__":',
  "    main()",
];
