// Composites scenes, their captions, and the reel's finishing passes.
// `render` is a pure function of time, so playback, scrubbing, and offline
// export agree. It imports no scene, so a preview can bundle just the one
// it shows (showreel-frames.mjs); reel.ts composes all of them.

import { CHAPTERS, DURATION, H, PALETTE, type ReelFacts, type Scene, sec, W } from "./bible";
import { grain, vignette } from "./fx";
import { clamp } from "./math";
import { drawCaptions, timeCaptions } from "./type";

export interface Reel {
  duration: number;
  chapters: { id: string; label: string; start: number; end: number }[];
  /** Draw the frame at `t` seconds into a canvas `pw` × `ph` device pixels. */
  render(ctx: CanvasRenderingContext2D, t: number, pw: number, ph: number): void;
}

export interface ReelOptions {
  /** Skip captions, grain, and vignette (for comparing raw scene frames). */
  raw?: boolean;
}

/**
 * A reel of `scenes`, in timeline order. Given every section's scene it is
 * the whole reel; given one, it draws that section on the reel's clock, so
 * times outside it fall to the nearest scene.
 */
export function composeReel(scenes: readonly Scene[], facts: ReelFacts | null, options: ReelOptions = {}): Reel {
  if (!scenes.length) throw new Error("a reel needs a scene");
  const sceneAt = (t: number): Scene => {
    for (const s of scenes) if (t < s.end) return s;
    return scenes[scenes.length - 1];
  };
  // Captions can depend on the numbers, so they are placed once per reel.
  const captions = scenes.flatMap((s) => timeCaptions(sec(s.id), s.captions?.(facts) ?? []));
  return {
    duration: DURATION,
    chapters: CHAPTERS,
    render(ctx, time, pw, ph) {
      const t = clamp(time, 0, DURATION - 1e-6);
      ctx.setTransform(pw / W, 0, 0, ph / H, 0, 0);
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = "source-over";
      ctx.fillStyle = PALETTE.bg;
      ctx.fillRect(0, 0, W, H);
      const s = sceneAt(t);
      const lt = t - s.start;
      ctx.save();
      s.draw(ctx, lt, { W, H, t, facts });
      ctx.restore();
      if (!options.raw) {
        // Darker toward the corners, but not over a terminal's window, a lit screen.
        vignette(ctx, W, H, 0.35, s.lit?.(lt, { facts }) ?? null);
        // Over the vignette, so a caption reads the same at the frame's edge;
        // under the grain, so it sits in the picture.
        drawCaptions(ctx, t, captions);
        grain(ctx, W, H, t, 0.07);
      }
      // Guard against a scene leaving the transform or blend mode dirty.
      ctx.setTransform(1, 0, 0, 1, 0, 0);
      ctx.globalAlpha = 1;
      ctx.globalCompositeOperation = "source-over";
    },
  };
}
