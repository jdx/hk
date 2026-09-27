// The beats of section 4, "Unstaged work is stashed" (storyboard §6.4),
// stash-local. The picture (scenes/stash.ts) moves on them and the score
// (score/stash.ts) reads them, so its cues follow the picture. Plain numbers
// only: no drawing code, so the score can import this module on its own.
// The blade's and the flying strip's beats (CUT, PEEL, FLY, INTO,
// SPARKS_AT) are stash-peel.ts's, next to the geometry they move.

/** The commit's pane squeezes into STRIP, settling on b1.5. */
export const PANE_IN = { from: 0, to: 1.5 } as const;
/**
 * The main.py card is launched from below the frame at `from`, reaches its
 * place still moving on `to` (the score's pat), carries on up a little and
 * is exactly home again on `settled`, well before the cut.
 */
export const CARD_IN = { from: 0.5, to: 1.5, settled: 1.875 } as const;
/** The tray rises in behind it, settled before its lid opens. */
export const TRAY_IN = { from: 1, to: 1.875 } as const;
/** The `unstaged` tag rides in with the card, all there as it lands; the sparks blow it away. */
export const TAG_IN = 1.125;
/** The lid opens on the cut, stays up for the flight, and slams on b3. */
export const LID = { open: 2, opened: 2.1875, shut: 2.875, slam: 3 } as const;
/** The pane's second row reports the stash as the lid slams. */
export const ROW_AT = 3;
/** What the linters see: a read band down the staged lines. */
export const READ = { from: 3.5, to: 5 } as const;
/**
 * The hold breathes: the lit gutter and squiggles pulse on the beats and the
 * card's shadow swells and settles on a two-beat sine, as if it floated,
 * from the end of the read band until b9.5, where all of it is exactly
 * still again.
 */
export const HOLD = { from: 4.5, full: 5.5, fade: 8.5, to: 9.5 } as const;
/**
 * A squiggle under each staged line ruff and ruff-format change, one per
 * eighth: `import os` (ruff removes it, F401), `def main()->None:`,
 * `print( 'ahoy' )` and `def hoist(sail:str)->str:` (ruff-format respaces
 * them). Each runs under the line's text, from its indent (col0) to its end
 * (col1), in MAIN_PY_STAGED's rows.
 */
export const SQUIGGLES = [
  { row: 0, col0: 0, col1: 9, at: 6 },
  { row: 1, col0: 0, col1: 17, at: 6.5 },
  { row: 2, col0: 4, col1: 19, at: 7 },
  { row: 4, col0: 0, col1: 25, at: 7.5 },
] as const;
/** How long a squiggle takes to draw on. */
export const SQUIGGLE_DRAW = 0.25;
/** Into the lanes: the strip leaves through the top… */
export const STRIP_OUT = { from: 9.5, to: 10.5 } as const;
/** …the card collapses into lane 3's label… */
export const CARD_OUT = { from: 10, to: 11 } as const;
/** …the other three labels type on, `each` apart, `rate` characters a beat… */
export const TYPE = { from: 10.375, each: 0.125, rate: 24 } as const;
/** …the four tracks draw, `each` apart, each over `dur`… */
export const TRACKS = { from: 11, each: 0.125, dur: 0.375 } as const;
/** …and the four open padlocks pop in, `each` apart, each over `dur`. */
export const LOCKS = { from: 11.5, each: 0.0625, dur: 0.25 } as const;
/** From here on the frame is the stash|lanes handoff. */
export const SETTLED = LOCKS.from + 3 * LOCKS.each + LOCKS.dur;
