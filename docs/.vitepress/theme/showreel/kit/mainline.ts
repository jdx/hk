// K6: the main branch (storyboard §4 K6), used by restore and catch: a line
// with the parent commit and the commit the reel makes, `feat: hoist the
// sails`, hashes from canon80 (the capture pins its commit dates, so they
// come out the same every time). `restore` lands the head dot on it; `catch`
// pans it left to make room for a commit that never lands.

import { PALETTE } from "../bible";
import { rgba } from "../color";
import { glow } from "../fx";
import { clamp, lerp } from "../math";
import { drawText, font, MONO } from "../type";

export const MAIN = {
  y: 600,
  x0: 160,
  x1: 1760,
  width: 3,
  dotR: 14,
  label: { text: "main", x: 160, y: 572 },
  parent: { hash: "f92f487", x: 1000, hashY: 660 },
  head: { hash: "ada2ca4", x: 1400, hashY: 560, message: "feat: hoist the sails", messageY: 508, glow: 40 },
} as const;

export type DotKind = "parent" | "head" | "slot";

/**
 * A commit on the line: the parent in text3; the head, the commit hk made,
 * in hk's cyan with a soft glow (not logo cyan, which is only the wordmark,
 * the hook, its line and hk's benchmark bar; storyboard §3); a slot, the
 * dashed empty ring where a commit would go. `glow` scales the head's glow,
 * radius and alpha (1, the default, is at rest).
 */
export function drawCommitDot(ctx: CanvasRenderingContext2D, x: number, kind: DotKind, o: { y?: number; alpha?: number; r?: number; glow?: number } = {}): void {
  const a = o.alpha ?? 1;
  if (a <= 0) return;
  const y = o.y ?? MAIN.y;
  const r = o.r ?? MAIN.dotR;
  ctx.save();
  ctx.globalAlpha *= a;
  if (kind === "slot") {
    ctx.fillStyle = PALETTE.bg;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fill();
    ctx.setLineDash([6, 6]);
    ctx.strokeStyle = PALETTE.text3;
    ctx.lineWidth = 2;
    ctx.stroke();
  } else {
    const g = o.glow ?? 1;
    if (kind === "head") glow(ctx, x, y, MAIN.head.glow * g, PALETTE.cyan, 0.35 * g);
    ctx.fillStyle = kind === "head" ? PALETTE.cyan : PALETTE.text3;
    ctx.beginPath();
    ctx.arc(x, y, r, 0, Math.PI * 2);
    ctx.fill();
  }
  ctx.restore();
}

export interface MainOptions {
  /** Horizontal offset, px: the history slides left, the line's right end stays at the margin. */
  pan?: number;
  /** All the text, 0..1. */
  labels?: number;
  /** The line drawn on from the left, 0..1. */
  line?: number;
  /** The parent and head dots, 0..1 each. */
  parent?: number;
  head?: number;
  /** The head's glow, 1 at rest (drawCommitDot's `glow`). */
  headGlow?: number;
  alpha?: number;
}

/**
 * The main line at y 600 with its two commits: the `main` label, `f92f487`
 * below its dot, and `ada2ca4` above the glowing head with its message
 * above that. With no options, the restore|catch handoff.
 */
export function drawMain(ctx: CanvasRenderingContext2D, o: MainOptions = {}): void {
  const a = o.alpha ?? 1;
  if (a <= 0) return;
  const pan = o.pan ?? 0;
  const labels = clamp(o.labels ?? 1);
  ctx.save();
  ctx.globalAlpha *= a;

  // History runs off the left edge as it pans; the branch's tip stays at the margin.
  const left = MAIN.x0 + pan;
  const right = lerp(left, MAIN.x1, clamp(o.line ?? 1));
  if (right > left) {
    ctx.fillStyle = PALETTE.text3;
    ctx.fillRect(left, MAIN.y - MAIN.width / 2, right - left, MAIN.width);
  }
  drawCommitDot(ctx, MAIN.parent.x + pan, "parent", { alpha: o.parent ?? 1 });
  drawCommitDot(ctx, MAIN.head.x + pan, "head", { alpha: o.head ?? 1, glow: o.headGlow });

  if (labels > 0) {
    const mono = font(40, 400, MONO);
    drawText(ctx, MAIN.label.text, MAIN.label.x + pan, MAIN.label.y, { font: mono, fill: rgba(PALETTE.cyan, labels) });
    drawText(ctx, MAIN.parent.hash, MAIN.parent.x + pan, MAIN.parent.hashY, { font: mono, fill: rgba(PALETTE.text2, labels), align: "center" });
    drawText(ctx, MAIN.head.hash, MAIN.head.x + pan, MAIN.head.hashY, { font: mono, fill: rgba(PALETTE.text1, labels), align: "center" });
    drawText(ctx, MAIN.head.message, MAIN.head.x + pan, MAIN.head.messageY, { font: font(40, 500), fill: rgba(PALETTE.text2, labels), align: "center" });
  }
  ctx.restore();
}
