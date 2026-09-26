// A commit hk can't fix: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 3 + 4 = 7 words need 4.5 beats; held 5 from b5 (line 2 needs 3, holds 4).
  { out: 10, lines: [{ in: 5, text: "Can't be fixed?" }, { in: 6, text: "hk blocks the commit." }] },
];

export const scene = stubScene(
  "catch",
  CAPTIONS,
  "When a finding can't be fixed automatically, the hook fails and there is no commit: hk's own hook catches it. The poster frame is beat 8.5.",
);
