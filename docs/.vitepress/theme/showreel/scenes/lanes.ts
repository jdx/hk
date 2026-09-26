// Steps in parallel: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 6 words: need 4, hold 4.
  { out: 6, lines: [{ in: 2, text: "Different files? Steps run at once." }] },
  // 5 words: need 3.5, hold 3.5; lands on the clamp's echo.
  { out: 12, lines: [{ in: 8.5, text: "Same file? Fixes take turns." }] },
];

export const scene = stubScene(
  "lanes",
  CAPTIONS,
  "A commit is a fix run, so every step takes write locks on its files. Steps on different files run at once, steps on the same file take turns, and depends orders ruff-format after ruff.",
);
