// Steps in hk.pkl: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions. */
export const CAPTIONS: readonly Caption[] = [
  // 5 words: need 3.5, hold 4.
  { out: 6.5, lines: [{ in: 2.5, text: "Builtins, from `prettier` to `zizmor`." }] },
  // 3 + 3 words: need 4, hold 4 (line 2 needs 2.5, holds 3.5).
  { out: 11.5, lines: [{ in: 7.5, text: "Configured in Pkl:" }, { in: 8, text: "typed and reusable." }] },
];

export const scene = stubScene(
  "config",
  CAPTIONS,
  "hk ships builtin configurations for many tools, and you choose steps in a typed Pkl file. The seven steps shown are exactly the ones the commit will run.",
);
