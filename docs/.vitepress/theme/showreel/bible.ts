// The reel's shared contract: the frame, the clock (re-exported from
// timeline.ts), the palette, and the shape of a scene. Scenes own everything
// between two bar lines; handoff.ts owns the frame on each bar line. Nothing
// here draws, so any module can import it without pulling in drawing code.

import type { ReelFacts } from "./facts";
import type { SectionId } from "./timeline";
import type { Caption } from "./type";

/** Logical frame size. Scenes draw in these units at any output resolution. */
export const W = 1920;
export const H = 1080;

// The clock lives in timeline.ts, which imports nothing, so the landing page
// can read the chapters without pulling in the drawing code.
export {
  BAR,
  BEAT,
  BPM,
  bar,
  beat,
  CHAPTERS,
  chaptersVtt,
  DURATION,
  type Section,
  type SectionId,
  SECTIONS,
  sec,
} from "./timeline";

/**
 * The stage (storyboard §3): side margins at x 160 and 1760, actors between
 * y 100 and FLOOR, and the captions' band below CAPTION_TOP (type.ts) kept
 * quiet while a caption is up.
 */
export const STAGE = { left: 160, right: 1760, top: 100, floor: 700 } as const;
/** Actors stand at or above this y. */
export const FLOOR = STAGE.floor;

/**
 * The site's dark theme, one step darker than the page so the video reads as
 * an object on it, with the colour roles every scene shares: cyan reads,
 * checks and is hk; warm writes, fixes and is code; green passed; red caught;
 * paper is the user's own unstaged work and the captions; logo cyan only for
 * the marks and hk's benchmark bar.
 */
export const PALETTE = {
  /** Deepest background: the icon background, behind `open` and `end` and the <video> letterbox. */
  night: "#071019",
  /** Default stage (docs dark --vp-c-bg-alt). */
  bg: "#0c151d",
  /** The page's own dark background. */
  page: "#101a23",
  /** Raised surfaces: cards, the tray, chrome bars (--vp-c-bg-soft). */
  surface: "#16232e",
  /** Chips, lane tracks, pills on a surface (--vp-c-bg-elv). */
  elevated: "#1b2a36",
  /** Hairlines and window edges (--vp-c-divider). */
  divider: "#2a3a46",
  /** Type (docs dark text tokens): 15.6, 9.6 and 6.6 : 1 on bg. */
  text1: "#e8edf0",
  text2: "#adbdc9",
  text3: "#8a9eac",
  /** The social card's warm off-white: captions, the end card, and the user's unstaged work. */
  paper: "#f4eee3",
  paperDim: "#c2b6a4",
  /** Brand cyan: reads, checks, strings, hk. */
  cyan: "#73d5df",
  cyanBright: "#8de1e8",
  cyanDeep: "#08768b",
  /** The logo stroke: only the wordmark, the icon hook, its line, and hk's benchmark bar. */
  logo: "#4adef0",
  /** The social cards' cyan. */
  social: "#4cc9f0",
  /** Specular highlight on the hook: glint and sparkle. */
  glint: "#ecfdff",
  /** Warm: writes, fixes, code (--hk-code-keyword dark). */
  warm: "#eac18e",
  warmBright: "#f5d9b3",
  warmDeep: "#c8955b",
  /** Soft fills, as the landing page's lanes. */
  cyanSoft: "rgba(115,213,223,0.10)",
  warmSoft: "rgba(234,193,142,0.12)",
  /** Bar fills, stronger than the page's so bars read at thumbnail size (storyboard §3). */
  cyanFill: "rgba(115,213,223,0.20)",
  warmFill: "rgba(234,193,142,0.22)",
  /** Passed and caught (hk's terminal green and red, Catppuccin Frappé). */
  green: "#a6d189",
  red: "#e78284",
  /** The same two under identity.md's names. */
  success: "#a6d189",
  error: "#e78284",
  /** The social card's warm rule. */
  rule: "#3d3540",
} as const;

/**
 * A terminal: Catppuccin Frappé foregrounds (the README demo's theme) on the
 * reel's own window colours, so the pane sits in the teal stage.
 */
export const TERM = {
  /** Window body, between bg and surface. */
  window: "#111c25",
  /** Chrome bar. */
  chrome: "#16232e",
  edge: "#2a3a46",
  /** Window buttons (VHS WindowBar Colorful). */
  dots: ["#ff4c4a", "#fcb900", "#00ce19"] as const,
  /** Default foreground and `cur/total`. */
  text: "#c6d0f5",
  /** Version, `by @jdx`, ` – <hook>`, the bar, ❯: 5.1 : 1, so detail sizes only. */
  dim: "#838ba7",
  subtle: "#a5adce",
  /** ✔ */
  green: "#a6d189",
  /** ✗ and ERROR */
  red: "#e78284",
  /** ⚠ */
  yellow: "#e5c890",
  /** The spinner and ⇢ */
  blue: "#8caaee",
  /** The bold `hk` in the header. */
  magenta: "#f4b8e4",
  cyan: "#81c8be",
  /** The shell's `$ `. */
  prompt: "#8caaee",
  /** Rosewater block cursor. */
  cursor: "#f2d5cf",
} as const;

/** The landing page's lanes, scaled up for the reel (storyboard K3). */
export const LANE = {
  track: PALETTE.elevated,
  check: { fill: PALETTE.cyanFill, stroke: PALETTE.cyan, text: PALETTE.cyan },
  fix: { fill: PALETTE.warmFill, stroke: PALETTE.warm, text: PALETTE.warm },
  /** A step queued behind a lock: no fill, a dashed outline. */
  waiting: { stroke: PALETTE.text3, text: PALETTE.text3, dash: [10, 8] as const },
} as const;

// The benchmark claims are read, checked and typed where the published run
// is parsed: facts.ts.
export type { ReelFacts };

export interface SceneEnv {
  W: number;
  H: number;
  /** Global time in seconds. */
  t: number;
  facts: ReelFacts | null;
}

export interface Scene {
  id: SectionId;
  /**
   * Global start and end, seconds: its section's, from `sec(id)`. The frame
   * at `end` belongs to the next scene.
   */
  start: number;
  end: number;
  /** Draw one frame. `lt` is local time, `t - start`. Paint the whole frame. */
  draw(ctx: CanvasRenderingContext2D, lt: number, env: SceneEnv): void;
  /**
   * The section's must-read captions, in its local beats. The reel draws
   * them over the scene in the lower third (type.ts), so a scene lists them
   * here and keeps that band quiet while they are up.
   */
  captions?(facts: ReelFacts | null): readonly Caption[];
  /**
   * The lit screen on the frame at `lt`, if there is one: a terminal's
   * window, which the reel's vignette leaves out.
   */
  lit?(lt: number): LitRect | null;
}

/**
 * A screen in the frame, logical px: a terminal's window, lit from within.
 * The vignette spares it, so the pane's colours read as the terminal's
 * wherever it stands.
 */
export interface LitRect {
  x: number;
  y: number;
  w: number;
  h: number;
  /** How far the vignette spares it, 0 to 1. */
  alpha: number;
}
