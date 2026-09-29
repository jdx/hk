// "No “fail, git add, and commit again”": the dance a hook that only checks
// puts you through when a formatter changes your files, and which hk skips
// by staging its fixes. Over restore's staged lanes, each step of it lands
// as it is sung and is struck through as it lands; the three go as the
// stash comes up.

import { BEAT, PALETTE } from "../bible";
import { rgba } from "../color";
import { roundedRect } from "../fx";
import { drawMono } from "../kit/card";
import { popIn } from "../kit/motion";
import { progress, smoothstep, swiftOut } from "../math";
import { drawCross } from "../scenes/catch-rig";
import { WORD } from "../type";
import { sungAt } from "./song";

/** The steps skipped, each landing on its word. `fail` is the hook's ✗. */
const STEPS = [
  { text: "pre-commit failed", fail: true, at: sungAt("verse-4.2", "fail") },
  { text: "$ git add -u", fail: false, at: sungAt("verse-4.2", "git") },
  { text: "$ git commit", fail: false, at: sungAt("verse-4.2", "commit") },
] as const;

/** Clear of restore's lanes (x 160–840) and its stash tray (from y 620). */
const X = 1000;
const ROWS = [250, 370, 490] as const;
const SIZE = 36;
const H = 72;
const PAD = 28;
/** The ✗'s cell before a failure's text. */
const CROSS_W = 44;

/** They go as the stash comes up. */
const GONE = [sungAt("verse-4.3", "Then") - 0.1, sungAt("verse-4.3", "Then") + 0.3] as const;

/** Draw the struck-out dance at song time `t`, over restore's frame. */
export function drawDance(ctx: CanvasRenderingContext2D, t: number): void {
  const out = 1 - smoothstep(GONE[0], GONE[1], t);
  if (out <= 0 || t < STEPS[0].at - WORD) return;
  STEPS.forEach((s, i) => {
    const land = s.at - WORD;
    if (t < land) return;
    const w = PAD * 2 + (s.fail ? CROSS_W : 0) + 0.6 * SIZE * s.text.length;
    const cy = ROWS[i];
    const scale = popIn(t, land, 5, 0.6);
    ctx.save();
    ctx.globalAlpha *= out * Math.min(1, progress(land, land + WORD, t) / 0.6);
    ctx.translate(X + w / 2, cy);
    ctx.scale(scale, scale);
    ctx.translate(-(X + w / 2), -cy);
    roundedRect(ctx, X, cy - H / 2, w, H, 12);
    ctx.fillStyle = PALETTE.surface;
    ctx.fill();
    ctx.strokeStyle = PALETTE.divider;
    ctx.lineWidth = 2;
    ctx.stroke();
    let x = X + PAD;
    if (s.fail) {
      drawCross(ctx, x + 14, cy - 2, 26, PALETTE.red, 0.14);
      x += CROSS_W;
    }
    drawMono(ctx, s.text, x, cy + SIZE * 0.34, SIZE, PALETTE.text2);
    // Struck through as it lands, left to right over a sixteenth.
    const strike = swiftOut(progress(s.at, s.at + BEAT / 4, t));
    if (strike > 0) {
      ctx.strokeStyle = rgba(PALETTE.red, 0.95);
      ctx.lineWidth = 5;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(X + 14, cy);
      ctx.lineTo(X + 14 + strike * (w - 28), cy);
      ctx.stroke();
    }
    ctx.restore();
  });
}
