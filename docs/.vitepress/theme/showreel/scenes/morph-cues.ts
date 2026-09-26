// When things happen in the morph (storyboard §6.10), in section-local
// seconds. The picture (scenes/morph.ts) is timed by these, and the score
// (score/morph.ts) can land its sounds on them: this module draws nothing
// and imports only the clock, so the score can import it.

import { BEAT } from "../timeline";

/** Local seconds of section beat `n`. */
const b = (n: number): number => n * BEAT;

/** The heartbeat (score/grooves.ts heartbeat): lub on b0 and b2, dub a sixteenth after each. */
export const LUB = [b(0), b(2)] as const;
export const DUB = [b(0.25), b(2.25)] as const;

/**
 * A light runs down bar i over the sixteenth from FILL[i]: down the hk bar
 * on the lub, then down each grey rival bar in turn, turning it hk's cyan.
 */
export const FILL = [b(0), b(0.25), b(0.5), b(0.75)] as const;
export const FILL_DUR = b(0.25);

/** Each bar lifts off as its light reaches its end, a sixteenth apart (the storyboard's stagger). */
export const LIFT = [b(0.25), b(0.5), b(0.75), b(1)] as const;

/**
 * Each bar lands on its stroke 1.25 beats after it lifts, a sixteenth apart:
 * the h's stem, the k's stem, the k's arm, and the pen at the end of the
 * leg's straight run. The four arrivals, a descending pluck each.
 */
export const LAND = [b(1.5), b(1.75), b(2), b(2.25)] as const;

/** The last bar is exactly the leg's straight run here, and the leg carries on from it. */
export const SWAP = LAND[3];

/** The h's shoulder and the rest of the k's leg write on over these. */
export const PEN = [b(2.25), b(3.5)] as const;

/** From here every frame is morph|end, the wordmark minus its barb, still. */
export const STILL = b(3.5);
