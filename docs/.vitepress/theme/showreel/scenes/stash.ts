// Unstaged work is stashed: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 4 + 7 = 11 words need 6.5 beats; held 6.5 from b3 (line 2 needs 4.5, holds 5.5). Line 2 is exactly 36 characters.
  { out: 9.5, lines: [{ in: 3, text: "Unstaged work is stashed," }, { in: 4, text: "so linters see only what you staged." }] },
];

export const scene = stubScene(
  "stash",
  CAPTIONS,
  "Partial commits: hk sets the unstaged line aside, so the tools see only what you staged.",
);
