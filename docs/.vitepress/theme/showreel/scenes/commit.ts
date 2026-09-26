// git commit runs hk: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 5 + 5 = 10 words need 6 beats; held 6 from b4.5 (line 2 needs 3.5, holds 5.5).
  { out: 10.5, lines: [{ in: 4.5, text: "After `hk install`, `git commit`" }, { in: 5, text: "runs hk before the commit." }] },
];

export const scene = stubScene(
  "commit",
  CAPTIONS,
  "You type the command you always type, and hk runs as the pre-commit hook.",
);
