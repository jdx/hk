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
/** A column draws in over this from its COLUMN_IN beat: title, file and track first, its padlock and line from a sixteenth in. */
export const COLUMN_DRAW = 1 / 2;

/**
 * hk check: ruff-format runs its diff command on src/main.py under a read
 * lock from b6.5, and fails on b7, since the diff is not empty: a red ✗,
 * and the lock springs open.
 */
export const CHECK_GO = 6.5;
export const CHECK_DONE = 7;
/** Its diff unfolds under it from the ✗, a row every 1/16 beat: eight rows, whole on b7.5. */
export const DIFF_IN = 7;
export const DIFF_ROW = 1 / 16;
/**
 * hk fix: ruff-format runs the same diff command under a read lock from
 * b7.5, as the check's diff is whole, and has its patch on b8: the read
 * lock lets go, and its own diff comes up under it, a row every 1/32 beat,
 * whole on b8.25.
 */
export const FIX_GO = 7.5;
export const FIX_PATCH = 8;
export const FIX_DIFF_ROW = 1 / 32;
/**
 * A sixteenth after the read lock lets go, the step takes the write lock,
 * as a lane's lock handed straight on does (kit/lanes.ts), and hk applies
 * the patch over b8.375–8.875: the removed rows are struck and fold away,
 * and the added rows settle into the file. ✔ on b9, and the lock springs
 * open.
 */
export const FIX_WRITE = 8.25;
export const APPLY = [8.375, 8.875] as const;
export const FIX_DONE = 9;

/** Both columns done. */
export const ALL_DONE = 9.5;
