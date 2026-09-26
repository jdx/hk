// hk: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 5 + 2 = 7 words need 4.5 beats; held 4.5 from b2.5 (line 2 needs 2, holds 4).
  { out: 7, lines: [{ in: 2.5, text: "Git hooks and project checks," }, { in: 3, text: "in parallel." }] },
];

export const scene = stubScene(
  "open",
  CAPTIONS,
  "Brand the reel and state what hk is: the hook drops, docks into the k, and the wordmark clicks into place.",
);
