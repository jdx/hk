// The exact frame on every bar line. The outgoing scene settles onto its
// boundary's frame by its last beat and the incoming scene starts from it at
// rest (a "hold"), or both evaluate the same function of global time through
// it (a "motion": the whip into `race`). The frame at a section's `end`
// belongs to the next section.
//
// Every frame is drawn only with the shared kit's own calls, the same calls
// the scenes make, so a scene that ends by making them lands on its handoff
// to the pixel; test/handoff-frames.test.ts holds every scene to that in
// Chromium. None reads the facts: the frame on a bar line is the same
// whatever the benchmarks say. The tray's shimmer and the whip read global
// time (env.t), so a hold that shows the tray is the same frame at the same t.

import { type LitRect, PALETTE, type SceneEnv, SECTIONS, type SectionId, sec } from "./bible";
import { drawChip } from "./kit/card";
import { drawFinalGantt, drawLanes } from "./kit/lanes";
import { drawLogo, LOGO_END, LOGO_FULL, LOGO_OPEN } from "./kit/logo";
import { drawMain } from "./kit/mainline";
import { commit, PROMPT_COMMIT } from "./kit/screens";
import { drawTerm, PANE_FULL, termLit } from "./kit/term";
import { drawTray } from "./kit/tray";
import { drawWhip } from "./whip";

type Adjacent<T extends readonly { id: string }[]> = T extends readonly [
  infer A extends { id: string },
  infer B extends { id: string },
  ...infer R extends { id: string }[],
]
  ? [`${A["id"]}|${B["id"]}`, ...Adjacent<[B, ...R]>]
  : [];
/** A bar line between two sections: `"<from>|<to>"`. */
export type BoundaryId = Adjacent<typeof SECTIONS>[number];

/** Every boundary in order, from open|config to morph|end. */
export const BOUNDARIES = SECTIONS.slice(1).map((s, i) => `${SECTIONS[i].id}|${s.id}` as BoundaryId);

export interface Handoff {
  id: BoundaryId;
  from: SectionId;
  to: SectionId;
  /** The bar line, global seconds: `sec(to).start`. */
  t: number;
  /**
   * hold: both scenes rest on this frame (the outgoing one settles onto it
   * by its last frame, the incoming one starts from it at rest). motion: a
   * shared move crosses the bar line (drawWhip); both scenes follow the
   * named function through it.
   */
  meet: "hold" | "motion";
  /** What is on the frame, in words. */
  note: string;
  /**
   * The lit screen on the frame (Scene.lit), which the vignette spares: both
   * scenes return it on their side of the bar line, so the vignette does not
   * jump there. Null where the frame has no screen.
   */
  lit: LitRect | null;
  /** Paint the whole frame. A motion handoff reads `env.t`. */
  draw(ctx: CanvasRenderingContext2D, env: SceneEnv): void;
}

/** Fill the whole frame. */
export const bg = (ctx: CanvasRenderingContext2D, env: SceneEnv, color: string = PALETTE.bg): void => {
  ctx.fillStyle = color;
  ctx.fillRect(0, 0, env.W, env.H);
};

/**
 * race|morph's capsules: round-capped, 56 px high, x 700–1420,
 * centred on four rows. `race` eases its bars into them and `morph` tweens
 * them into the end card's strokes, so both draw them with drawCapsule.
 */
export const CAPSULES = {
  x0: 700,
  x1: 1420,
  h: 56,
  rows: [270, 390, 510, 630],
  /** The first is hk's bar, in logo cyan; the rest are the other tools, text3 at 0.35. */
  color: [PALETTE.logo, PALETTE.text3, PALETTE.text3, PALETTE.text3],
  alpha: [1, 0.35, 0.35, 0.35],
} as const;

/**
 * One capsule: a round-capped segment `width` px thick whose cap centres are
 * `a` and `b` (the ink reaches `width / 2` past each), as the logo's strokes
 * are drawn, so a capsule can turn into one of them without a seam. `color`
 * should be opaque; fade with `alpha`.
 */
export function drawCapsule(
  ctx: CanvasRenderingContext2D,
  a: { x: number; y: number },
  b: { x: number; y: number },
  width: number,
  color: string,
  alpha = 1,
): void {
  if (!(alpha > 0) || !(width > 0)) return;
  ctx.save();
  ctx.globalAlpha *= alpha;
  ctx.strokeStyle = color;
  ctx.lineWidth = width;
  ctx.lineCap = "round";
  ctx.beginPath();
  ctx.moveTo(a.x, a.y);
  ctx.lineTo(b.x, b.y);
  ctx.stroke();
  ctx.restore();
}

/** Capsule `i` of race|morph where the handoff puts it. */
export function capsuleAt(i: number): { a: { x: number; y: number }; b: { x: number; y: number }; width: number; color: string; alpha: number } {
  const { x0, x1, h, rows } = CAPSULES;
  const y = rows[i];
  return { a: { x: x0 + h / 2, y }, b: { x: x1 - h / 2, y }, width: h, color: CAPSULES.color[i], alpha: CAPSULES.alpha[i] };
}

/** The four capsules of race|morph, at rest. */
export function drawCapsules(ctx: CanvasRenderingContext2D): void {
  for (let i = 0; i < CAPSULES.rows.length; i++) {
    const c = capsuleAt(i);
    drawCapsule(ctx, c.a, c.b, c.width, c.color, c.alpha);
  }
}

/** morph|end's wordmark: every stroke but the barb, which `end` draws on its downbeat (the click). */
export const END_STROKES: readonly number[] = [1, 1, 1, 1, 1, 0];

/** The pre-commit pane at 0/7 (commit|stash): the typed command, then capture frame 1, the header and the four staged files. */
export const COMMIT_STASH_LINES: readonly string[] = [PROMPT_COMMIT, ...commit[1]];

const LIST: Omit<Handoff, "from" | "to" | "t">[] = [
  {
    id: "open|config",
    meet: "hold",
    note: "The hk wordmark, every stroke drawn, at LOGO_OPEN (centre 960, 400; 280 px) in logo cyan on bg. No line, hook, glint or sparkle.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env);
      drawLogo(ctx, LOGO_OPEN, LOGO_FULL);
    },
  },
  {
    id: "config|commit",
    meet: "hold",
    note: "The hk.pkl chip (HKPKL_CHIP, x 850–1070, y 120–180) alone on bg.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env);
      drawChip(ctx);
    },
  },
  {
    id: "commit|stash",
    meet: "hold",
    note: "PANE_FULL with the prompt row, the pre-commit header at 0/7 and the ✔ files row (4 staged files). No cursor.",
    lit: termLit(PANE_FULL),
    draw(ctx, env) {
      bg(ctx, env);
      drawTerm(ctx, PANE_FULL, COMMIT_STASH_LINES, { t: env.t });
    },
  },
  {
    id: "stash|lanes",
    meet: "hold",
    note: "Four empty file lanes (labels, tracks, open padlocks at x 600) and the stash tray, closed, holding the unstaged line.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env);
      drawLanes(ctx);
      drawTray(ctx, { holding: true, t: env.t });
    },
  },
  {
    id: "lanes|restore",
    meet: "hold",
    note: "The finished Gantt (drawFinalGantt: every fix bar, padlocks open, a ✔ per lane) and the closed tray. No playhead, chips or detail.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env);
      drawFinalGantt(ctx);
      drawTray(ctx, { holding: true, t: env.t });
    },
  },
  {
    id: "restore|catch",
    meet: "hold",
    note: "The main line at pan 0 with its labels: the f92f487 parent and the glowing ada2ca4 head, feat: hoist the sails.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env);
      drawMain(ctx);
    },
  },
  {
    id: "catch|everywhere",
    meet: "hold",
    note: "The hk.pkl chip alone on bg, identical to config|commit.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env);
      drawChip(ctx);
    },
  },
  {
    id: "everywhere|race",
    meet: "motion",
    note: "The whip (whip.ts drawWhip of global t): both scenes draw it; on the bar line it is streaks alone on bg, at their peak.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env);
      drawWhip(ctx, env.t);
    },
  },
  {
    id: "race|morph",
    meet: "hold",
    note: "Four neutral capsules, x 700–1420, 56 px high, on rows 270–630: the first logo cyan, the rest text3 at 0.35. No text, whatever the facts.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env);
      drawCapsules(ctx);
    },
  },
  {
    id: "morph|end",
    meet: "hold",
    note: "The wordmark minus its barb at LOGO_END (centre 1420, 330; 440 px) on night. No glint.",
    lit: null,
    draw(ctx, env) {
      bg(ctx, env, PALETTE.night);
      drawLogo(ctx, LOGO_END, END_STROKES);
    },
  },
];

/** Every handoff by its boundary. */
export const HANDOFFS = Object.fromEntries(
  LIST.map((h) => {
    const [from, to] = h.id.split("|") as [SectionId, SectionId];
    return [h.id, { ...h, from, to, t: sec(to).start }];
  }),
) as Record<BoundaryId, Handoff>;

/** Paint boundary `id`'s bar-line frame. */
export function drawHandoff(ctx: CanvasRenderingContext2D, id: BoundaryId, env: SceneEnv): void {
  HANDOFFS[id].draw(ctx, env);
}

/** The handoff a section starts from, or null for the first. */
export function handoffIn(id: SectionId): Handoff | null {
  return Object.values(HANDOFFS).find((h) => h.to === id) ?? null;
}

/** The handoff a section ends on, or null for the last. */
export function handoffOut(id: SectionId): Handoff | null {
  return Object.values(HANDOFFS).find((h) => h.from === id) ?? null;
}
