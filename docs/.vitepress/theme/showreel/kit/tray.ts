// K5: the stash tray (storyboard §4 K5), used by stash, lanes and restore.
// A small box at the lanes' lower right where hk keeps the unstaged line
// while the steps run: its lid opens on a hinge at the left to take the
// line or give it back, and a paper sliver shows in the lid's slot while it
// holds it. The sliver's shimmer runs on global time, so it does not jump
// on the bar lines the tray sits across.

import { PALETTE } from "../bible";
import { mix, rgba } from "../color";
import { roundedRect } from "../fx";
import { clamp, DEG, progress } from "../math";
import { drawText, font, MONO } from "../type";

/** The tray, logical px: the box, its 18 px lid, the slot the held line shows in, and the label. */
export const TRAY = {
  x: 1520,
  y: 620,
  w: 240,
  h: 80,
  r: 12,
  /** The lid is the box's top strip, hinged at its lower left. */
  lidH: 18,
  /** Fully open, the lid stands at this angle. */
  lidOpen: -35,
  /** The held line's sliver. */
  sliver: { x0: 1540, x1: 1740, y0: 626, y1: 632 },
  label: { text: "stash", x: 1548, y: 676 },
} as const;

export interface TrayOptions {
  /** 0 shut, 1 open (the lid at −35°). */
  lid?: number;
  /** The unstaged line is inside: a paper sliver in the slot. A number fades it. */
  holding?: boolean | number;
  /** Global seconds, for the sliver's shimmer. */
  t?: number;
  alpha?: number;
}

/** One shimmer pass along the sliver per bar and a half, on the reel's clock. */
const SHIMMER_PERIOD = 3;

/** The tray: surface, 1 px divider, radius 12, `stash` in mono 40 px warm. */
export function drawTray(ctx: CanvasRenderingContext2D, o: TrayOptions = {}): void {
  const a = o.alpha ?? 1;
  if (a <= 0) return;
  const { x, y, w, h, r, lidH, sliver } = TRAY;
  const lid = clamp(o.lid ?? 0);
  const holding = typeof o.holding === "number" ? clamp(o.holding) : o.holding ? 1 : 0;
  ctx.save();
  ctx.globalAlpha *= a;

  roundedRect(ctx, x, y, w, h, r);
  ctx.fillStyle = PALETTE.surface;
  ctx.fill();
  ctx.strokeStyle = PALETTE.divider;
  ctx.lineWidth = 1;
  ctx.stroke();
  // The mouth under the lid: dark, so the slot and an open lid show into it.
  ctx.fillStyle = PALETTE.night;
  roundedRect(ctx, x + 8, y + 3, w - 16, lidH - 3, 4);
  ctx.fill();

  if (holding > 0) {
    ctx.save();
    ctx.globalAlpha *= holding;
    // A shade under paper, so the highlight that travels it can be seen.
    ctx.fillStyle = mix(PALETTE.paper, PALETTE.paperDim, 0.35);
    ctx.fillRect(sliver.x0, sliver.y0, sliver.x1 - sliver.x0, sliver.y1 - sliver.y0);
    const u = (((o.t ?? 0) / SHIMMER_PERIOD) % 1 + 1) % 1;
    const span = sliver.x1 - sliver.x0;
    const cx = sliver.x0 - 60 + u * (span + 120);
    const grad = ctx.createLinearGradient(cx - 50, 0, cx + 50, 0);
    grad.addColorStop(0, rgba("#ffffff", 0));
    grad.addColorStop(0.5, rgba("#ffffff", 0.9));
    grad.addColorStop(1, rgba("#ffffff", 0));
    ctx.fillStyle = grad;
    ctx.fillRect(sliver.x0, sliver.y0, span, sliver.y1 - sliver.y0);
    ctx.restore();
  }

  // The lid, with its slot cut through, swung up on its hinge.
  ctx.save();
  ctx.translate(x, y + lidH);
  ctx.rotate(TRAY.lidOpen * DEG * lid);
  ctx.translate(-x, -(y + lidH));
  ctx.beginPath();
  ctx.moveTo(x, y + lidH);
  ctx.lineTo(x, y + r);
  ctx.arcTo(x, y, x + r, y, r);
  ctx.lineTo(x + w - r, y);
  ctx.arcTo(x + w, y, x + w, y + r, r);
  ctx.lineTo(x + w, y + lidH);
  ctx.closePath();
  ctx.rect(sliver.x0 - 4, sliver.y0 - 2, sliver.x1 - sliver.x0 + 8, sliver.y1 - sliver.y0 + 4);
  ctx.fillStyle = PALETTE.surface;
  ctx.fill("evenodd");
  ctx.strokeStyle = PALETTE.divider;
  ctx.lineWidth = 1;
  ctx.stroke();
  // Lifted, the lid's underside catches a little light.
  if (lid > 0) {
    ctx.fillStyle = rgba(PALETTE.text3, 0.12 * progress(0, 0.3, lid));
    ctx.fill("evenodd");
  }
  ctx.restore();

  drawText(ctx, TRAY.label.text, TRAY.label.x, TRAY.label.y, { font: font(40, 400, MONO), fill: PALETTE.warm });
  ctx.restore();
}
