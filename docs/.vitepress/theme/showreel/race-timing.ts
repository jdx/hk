// When the benchmark race's bars leave the axis, when each one stops, and
// when its captions land, in race-local beats. The picture (scenes/race.ts
// and race-chart.ts) and the score (score/race.ts) both read this, so each
// ding lands on the frame its bar stops and the roll call on the note's
// landing. It draws nothing, so the score can import it.

import { claimLine, type Race, type ReelFacts } from "./facts";
import { BEAT } from "./timeline";
import { readingTime, wordCount } from "./type";

/** The claim's second line. */
export const CLAIM_TAIL = "than the fastest other tool.";
/** The hold's caption, which wipes on NOTE_OUT. */
export const NOTE = "Timed only when the files are right.";
/** The last caption wipes here, its wipe done by REST (scenes/race-chart.ts). */
export const NOTE_OUT = 15.5;

export interface RaceRun {
  race: Race;
  /** The beat every bar leaves the axis together. */
  start: number;
  /**
   * The beat each bar stops, by subject key. A bar's run is proportional to
   * its median, so the slowest tool stops at `start + SPAN` (2.5 beats).
   * Stops are not quantized: two close medians must not stop on the same
   * frame.
   */
  stops: Readonly<Record<string, number>>;
  /** Subject keys in finishing order, fastest first. */
  order: readonly string[];
  /** The claim: its first line lands on `in`, its tail half a beat later, and both wipe on `out`. */
  claim: { in: number; out: number };
  /** The beat NOTE lands on. */
  note: number;
}

/** The race runs from b1 over 2.5 beats. */
const START = 1;
const SPAN = 2.5;

/** Beats a caption must hold to be read: words / 2 + 1 at 120 BPM, a multiple of half a beat. */
const need = (words: number): number => readingTime(words) / BEAT;

/**
 * The captions' beats. The claim lands on b2.5, or later, so that its ratio
 * (the line's second-to-last word, rising a quarter beat before the line
 * lands) starts to rise only once the fastest other tool's bar has stopped
 * (hk's stopped before it): half a beat after that stop, on the half-beat
 * grid. It holds to b8.5, past the bottom detail's rise on b8, or longer if
 * its words need it. The note lands on b9.75, or a beat after the claim
 * starts to wipe if that is later, so its first word (of seven, a 32nd
 * apart) rises only once the claim is leaving; it holds its 4.5 beats by
 * NOTE_OUT, so it lands by b11 and the claim wipes by b10.
 *
 * The rival stops by b3.5, so the claim lands by b4 and holds 6 beats by
 * b10: every claim up to 10 words with its tail, which "Fix every file" and
 * "Check every file" are (the commit's is 8). One of 11 to 13 words lands
 * earlier, as late as b10 allows, and its ratio may then rise before the
 * rival's bar stops. That is the limit: past 13 words (a title of seven
 * words or more, such as "Fix all of it in a repo", which a 36-character
 * claim line still allows) the claim needs more than 7.5 beats even from
 * b2.5, so it wipes after b10 and the note, landing after b11, holds less
 * than its 4.5 beats before NOTE_OUT. facts.ts does not reject such a title; captions.test.ts
 * checks the rule for every race the frozen runs back, for the live run's,
 * and for a made-up claim of 13 words.
 */
function captionBeats(race: Race, rivalStop: number): Pick<RaceRun, "claim" | "note"> {
  const claimNeed = need(wordCount(claimLine(race)) + wordCount(CLAIM_TAIL));
  const lands = Math.max(2.5, Math.min(Math.ceil(2 * (rivalStop + 0.5)) / 2, 10 - claimNeed));
  const out = Math.max(8.5, lands + claimNeed);
  return { claim: { in: lands, out }, note: Math.max(9.75, out + 1) };
}

/** The race the facts back, with its timing, or null when they back none. */
export function raceRun(f: ReelFacts | null): RaceRun | null {
  const race = f?.best;
  if (!race) return null;
  const slowest = Math.max(...race.rows.map((r) => r.median));
  const stops = Object.fromEntries(race.rows.map((r) => [r.key, START + (SPAN * r.median) / slowest]));
  return {
    race,
    start: START,
    stops,
    order: [...race.rows].sort((a, b) => a.median - b.median).map((r) => r.key),
    ...captionBeats(race, stops[race.claim.rival.key]),
  };
}
