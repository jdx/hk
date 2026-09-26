// Commit, terminal, CI: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 4 + 3 words: need 4.5, hold 4.5.
  { out: 6, lines: [{ in: 1.5, text: "One set of steps:" }, { in: 2.5, text: "commit, terminal, CI." }] },
  // 4 + 3 words: need 4.5, hold 4.5; its first word rises on b6, as the caption above starts to leave.
  { out: 11, lines: [{ in: 6.5, text: "Checks share a file." }, { in: 7, text: "Fixes take turns." }] },
];

export const scene = stubScene(
  "everywhere",
  CAPTIONS,
  "The same steps run in the pre-commit hook, from hk fix in your terminal, and from hk check --all in CI. Checks share a file; fixes take turns.",
);
