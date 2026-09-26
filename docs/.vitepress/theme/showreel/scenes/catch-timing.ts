// The beats of section 7, "A commit hk can't fix" (storyboard §6.7), in
// section-local beats, the inset's frame table, and the card's swing. The
// picture (scenes/catch.ts) moves on them and the score (score/catch.ts)
// reads them, so the creaks, plucks and ratchets land on the frames they
// belong to. Plain numbers only: no drawing code, so the score can import
// this module on its own.

import { smoothstep, TAU } from "../math";
import { BEAT } from "../timeline";

/** The picture's beats, catch-local. */
export const BEATS = {
  /** The main line has panned, and the `main` label sits over the head it still names. */
  panned: 1,
  /** The inset fades in. */
  insetIn: 0.5,
  /** The card enters at the left edge, cruises, brakes from `brake` and stops on `stop`, the ✗. */
  ride: 1,
  brake: 2.2,
  stop: 3,
  /** The hook whips down from the top edge, and snags the loop. */
  drop: [3.5, 3.875],
  snag: 4,
  /** Heave, ho: two tugs, each over a sixteenth. */
  tugs: [4.5, 5],
  tugLen: 0.25,
  /** The yank lifts the card off the line; it hangs from `swing`. */
  yank: 5.25,
  swing: 6,
  /** Light runs down the hook as it holds the card up. */
  glint: 6.25,
  /** The line reels the card out of the top. */
  reel: [10, 11.5],
  /** The chip comes up, and the frame is catch|everywhere from `hold`. */
  chip: 11.25,
  hold: 11.75,
} as const;

/** When the inset shows each of blocked's frames 0–13, holding each until the next (storyboard §6.7 table). */
export const FRAME_BEATS = [1, 1.25, 1.5, 1.75, 2, 2.125, 2.25, 2.375, 2.5, 2.625, 2.75, 3, 3.125, 3.25] as const;

/**
 * The swing, degrees, positive clockwise about the anchor above the frame,
 * at catch-local second `lt`: θ = A·e^(−d/2 beats)·sin(2πd/2 beats) from
 * the swing, eased in over the yank. A is the storyboard's 7° but for the
 * first lobe, held down to about 3.8° (A = 7°·(1 − 0.6·e^(−d/0.75 beats)))
 * so the card's left edge stays 30 px clear of the inset at the first apex
 * (b6.5); from the second apex on it is within 0.3° of 7°'s, and the
 * poster's apex (b8.5) is still θ ≈ 2.0°.
 */
export function swingAt(lt: number): number {
  const ramp = smoothstep(5.55 * BEAT, 6.1 * BEAT, lt);
  if (ramp <= 0) return 0;
  const d = lt - BEATS.swing * BEAT;
  const amp = 7 * (1 - 0.6 * Math.exp(-Math.max(0, d) / (0.75 * BEAT)));
  return amp * Math.exp(-d / (2 * BEAT)) * Math.sin((TAU * d) / (2 * BEAT)) * ramp;
}
