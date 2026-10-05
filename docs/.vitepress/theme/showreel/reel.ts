// The whole reel: every section's scene, composited (compose.ts). The video
// renderer (showreel-video.mjs) imports it, and the tests; the landing page
// only plays the rendered file.

import type { ReelFacts } from "./bible";
import { composeReel, type Reel, type ReelOptions } from "./compose";
import { scenes } from "./scenes";
import { sec } from "./timeline";

export type { Reel, ReelOptions } from "./compose";

/**
 * The video's poster frame: `catch` at beat 8.5 (40.25 s), with hk's hook
 * holding the blocked commit aloft at the top of its swing and the caption
 * "Can't be fixed? / hk blocks the commit." fully landed. It is the same
 * with and without benchmark data, and repeats nothing the hero shows.
 */
export const POSTER_TIME = sec("catch").beat(8.5);

export function createReel(facts: ReelFacts | null, options: ReelOptions = {}): Reel {
  return composeReel(scenes, facts, options);
}

// Re-exported for the video renderer.
export { factsFromBenchmarks } from "./facts";
export { resetTypeCache } from "./type";
