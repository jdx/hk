// The beats of section 8, "Commit, terminal, CI" (storyboard §6.8), in
// section-local beats. The picture (scenes/everywhere.ts) moves on them and
// the score (score/everywhere.ts) can import them, so each hit, pluck and
// padlock click lands on the frame it belongs to. Plain numbers only: no
// drawing code, so the score can import this module on its own.

/** A panel's flight out of the chip: a 32nd. */
export const FLY = 1 / 8;
/**
 * Each panel slams onto the floor: panels 2 and 3 on b1 and b2. Panel 1
 * lands a 32nd late, on b0.125, because it cannot leave the chip before
 * the section starts (the section's first frame is the chip alone).
 */
export const IMPACT = [0.125, 1, 2] as const;
/** Each panel leaves the chip (the chip kicks) a flight before it lands. */
export const LAUNCH = IMPACT.map((i) => i - FLY);

/** After a panel lands, its rows slide in one per 32nd from ROWS_FROM, each over ROW_EACH. */
export const ROWS_FROM = 1 / 4;
export const ROW_EACH = 1 / 8;
/** When row `row` of panel `panel` starts to slide in. */
export const rowStart = (panel: number, row: number): number => IMPACT[panel] + ROWS_FROM + row * ROW_EACH;
/** When it has slid in: its ✔ flares on this beat. */
export const rowLands = (panel: number, row: number): number => rowStart(panel, row) + ROW_EACH;
/**
 * Each panel's eight ✔ ticks: rows 1–8 of its screen (row 0 is the
 * header), landing one per 32nd from impact + 1/2 to impact + 11/8.
 */
export const TICKS: readonly (readonly number[])[] = IMPACT.map((_, i) => Array.from({ length: 8 }, (_, k) => rowLands(i, k + 1)));

/** A pulse leaves the chip down all three connectors on each eighth, and reaches the panels a 16th later. */
export const PULSES = [3, 3.5, 4, 4.5] as const;
export const PULSE_FLY = 1 / 4;
/** The panels roll up into the chip, left to right, a 32nd apart, each over FOLD_LEN. */
export const FOLD = [5.5, 5.625, 5.75] as const;
export const FOLD_LEN = 3 / 8;
/** The chip fades out after the panels. */
export const CHIP_OUT = [5.875, 6.25] as const;
/** The two columns draw in, left then right, settled before their first steps. */
export const COLUMN_IN = [6, 6.125] as const;

/** Checks: all three start together on b6.5 (the read lock snaps shut). */
export const CHECK_GO = 6.5;
/**
 * When each check's read ends and its ✔ lands, by row (prettier,
 * trailing-whitespace, newlines): newlines first and prettier last, as
 * `hk check --all` finished them, all on the 16th grid and done by b8.
 */
export const CHECK_DONE = [8, 7.25, 7] as const;

/**
 * Fixes: when each holds the write lock, by row (prettier,
 * trailing-whitespace, newlines), one per beat in the commit run's order.
 * The first takes it on b7; each lets it go on the beat (its ✔, the lock
 * springs open) and the next takes it a 16th later (the lock shuts, its
 * pill starts), the lanes' rule. The last lets it go on b10.
 */
export const FIX_HOLDS = [
  [7, 8],
  [8.25, 9],
  [9.25, 10],
] as const;

/** Every row of both columns done. */
export const ALL_DONE = 10;
