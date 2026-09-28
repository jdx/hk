// The beats of section 5, "Fixers in parallel" (storyboard §6.5), lanes-local.
// The picture (scenes/lanes.ts, which re-exports BEATS) moves on them and
// the score (score/lanes.ts) reads them, so its cues follow the picture.
// Plain numbers only: no drawing code, so the score can import this module
// on its own. test/lanes.test.ts holds each to the kit's SCHEDULE.

/**
 * The picture's beats, lanes-local. Every cue here lands on the half-beat
 * grid or a sixteenth off it.
 */
export const BEATS = {
  /** The dock chips slide in, the waiting chip pops onto lane 3, the bracket draws. */
  queue: 0.5,
  /**
   * All hands: the playhead drops in, prettier and ruff start writing their
   * files, shfmt and shellcheck start reading scripts/deploy.sh together,
   * and all four padlocks shut at once, deploy.sh's with two readers.
   */
  go: 1,
  /** shfmt has its patch: it lets its read lock go and waits, holding nothing, for shellcheck to finish reading. */
  shfmtPatch: 2,
  /** shellcheck ✔: its diff was empty, so it never takes a write lock. */
  shellcheckDone: 3,
  /** deploy.sh is free: the key hops to shfmt, whose write lock shuts, and hk applies its patch. */
  shfmtWrite: 3.25,
  /** shfmt ✔ writes deploy.sh. */
  shfmtDone: 4,
  /** ruff ✔ writes main.py… */
  ruffDone: 4.5,
  /** …and sends a spark along the depends bracket… */
  dependsSpark: 4.5,
  /** …which lands: the bracket snaps taut, and ruff-format starts reading main.py. */
  depends: 5,
  /** ruff-format has its patch: it trades its read lock for main.py's write lock. */
  ruffFormatWrite: 7.75,
  /** ruff-format ✔ writes main.py: hk has applied its patch. */
  ruffFormatDone: 8.5,
  /** prettier ✔ writes README.md and src/app.ts and lets them go; trailing-whitespace and newlines drop from the dock together. */
  clamp: 10,
  /** They land across all four lanes as readers: every padlock shuts cyan with two pips (the kit's lockedFrom). */
  slam: 10.25,
  /** Both ✔, having found nothing to fix. */
  passesDone: 12,
  /** The ✔ cascade down x 1730, a lane per sixteenth. */
  cascade: [12, 12.25, 12.5, 12.75],
  /** Once the run is done, a light crosses the finished chart: from, to. */
  glint: [13, 14.25],
  /** The detail line wipes. */
  detailOut: 14.5,
} as const;
