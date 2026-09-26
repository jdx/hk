// Fixes staged, edits back: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 3 + 4 = 7 words: need 4.5, hold 4.5 (line 2 needs 3, holds 3).
  { out: 6.5, lines: [{ in: 2, text: "Fixes are staged." }, { in: 3.5, text: "Your edits come back." }] },
  // 6 words: need 4, hold 4.
  { out: 11.5, lines: [{ in: 7.5, text: "The commit gets the fixed version." }] },
];

export const scene = stubScene(
  "restore",
  CAPTIONS,
  "The fixes are staged, your unstaged line comes back into the fixed file, and the commit lands with the fixed version.",
);
