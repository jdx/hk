// Benchmarks: a placeholder until the real scene replaces it (stub.ts). The
// captions are the storyboard's (§6.9): each race the facts back states its
// ratio against the fastest other tool; with one, the second slot says why
// the other is missing; with none (F0), a line with no number at all.

import { claimLine, type Race, races, type ReelFacts } from "../facts";
import type { Caption } from "../type";
import { stubScene } from "./stub";

/** The section's must-read captions under `f`. */
export function captions(f: ReelFacts | null): Caption[] {
  const both = races(f);
  const claim = (r: Race, at: number, out: number): Caption => ({
    out,
    lines: [
      { in: at, text: claimLine(r) },
      { in: at + 0.5, text: "than the fastest other tool." },
    ],
  });
  // 10 words each: need 6, hold 6.
  if (both.length === 2) return [claim(both[0], 2.5, 8.5), claim(both[1], 9.5, 15.5)];
  // 7 words: need 4.5, hold 5.75.
  if (both.length === 1) return [claim(both[0], 2.5, 8.5), { out: 15.5, lines: [{ in: 9.75, text: "Timed only when the files are right." }] }];
  // 5 words: need 3.5, hold 4. No digits.
  return [{ out: 7, lines: [{ in: 3, text: "Independent steps run in parallel." }] }];
}

export const scene = stubScene(
  "race",
  captions,
  "The published results, stated exactly as the benchmarks page states them: Fix every file and Check every file, hk against the fastest other tool. The commit scenario is on the page, not in the reel.",
);
