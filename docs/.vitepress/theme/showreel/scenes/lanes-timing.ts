// The beats of section 5, "Steps in parallel" (storyboard §6.5), lanes-local.
// The picture (scenes/lanes.ts, which re-exports BEATS) moves on them and
// the score (score/lanes.ts) reads them, so its cues follow the picture.
// Plain numbers only: no drawing code, so the score can import this module
// on its own.

/**
 * The picture's beats, lanes-local. Every cue here lands on the half-beat
 * grid or a sixteenth off it.
 */
export const BEATS = {
  /** The dock chips slide in, the waiting chips pop, the bracket draws. */
  queue: 0.5,
  /** The playhead drops in; prettier, ruff and shfmt start; four padlocks shut. */
  go: 1,
  /** shfmt ✔ and deploy.sh's padlock opens; the key hops to shellcheck. */
  shfmtDone: 2.5,
  /** shellcheck holds deploy.sh: the key lands on its chip and its padlock shuts again. */
  shellcheckLock: 2.75,
  ruffDone: 3.5,
  /** ruff ✔ sends a spark along the depends bracket… */
  dependsSpark: 3.5,
  /** …and lands: the bracket snaps taut, ruff-format starts and main.py locks. */
  depends: 4,
  shellcheckDone: 5,
  ruffFormatDone: 6.5,
  /** The docked steps knock on their locks. */
  knocks: [6.5, 7, 7.5, 8.5, 9, 9.5],
  /** prettier ✔; README.md and src/app.ts flash warm; trailing-whitespace, wound up since b7.75, dives. */
  clamp: 8,
  /** trailing-whitespace lands across all four lanes; every padlock shuts (the kit's lockedFrom). */
  slam: 8.25,
  twDone: 10,
  /** newlines lands across all four lanes. */
  slam2: 10.25,
  newlinesDone: 12,
  /** The ✔ cascade down x 1730, a lane per sixteenth. */
  cascade: [12, 12.25, 12.5, 12.75],
  /** Once the run is done, a light crosses the finished chart: from, to. */
  glint: [13, 14.25],
  /** The detail line wipes. */
  detailOut: 14.5,
} as const;
