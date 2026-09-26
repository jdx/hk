// Morph: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6, §7.1), in section-local beats.

import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions (none: the picture carries it). */
export const CAPTIONS: readonly Caption[] = [];

export const scene = stubScene(
  "morph",
  CAPTIONS,
  "Close the argument by turning the bars into the mark: four capsules tween into strokes of the hk logo.",
);
