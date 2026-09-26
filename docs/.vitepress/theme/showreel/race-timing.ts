// When the benchmark race's bars leave the axis and when each one stops, in
// race-local beats. The picture (scenes/race.ts) and the score
// (score/race.ts) both read this, so each ding lands on the frame its bar
// stops. It draws nothing, so the score can import it.

import { type Race, races, type ReelFacts } from "./facts";

export interface RaceRun {
  race: Race;
  /** The beat every bar leaves the axis together. */
  start: number;
  /**
   * The beat each bar stops, by subject key. A bar's run is proportional to
   * its median, so the slowest tool stops at `start + span`. Stops are not
   * quantized: two close medians must not stop on the same frame.
   */
  stops: Readonly<Record<string, number>>;
  /** Subject keys in finishing order, fastest first. */
  order: readonly string[];
}

// Storyboard §6.9: the first race the facts back runs from b1 over 2.5
// beats; a second (check every file) runs from b9 over one beat.
const WINDOWS = [
  { start: 1, span: 2.5 },
  { start: 9, span: 1 },
] as const;

/** The races the facts back, in the order the section runs them, with their timing. */
export function raceRuns(f: ReelFacts | null): RaceRun[] {
  return races(f).map((race, i) => {
    const { start, span } = WINDOWS[i];
    const slowest = Math.max(...race.rows.map((r) => r.median));
    return {
      race,
      start,
      stops: Object.fromEntries(race.rows.map((r) => [r.key, start + (span * r.median) / slowest])),
      order: [...race.rows].sort((a, b) => a.median - b.median).map((r) => r.key),
    };
  });
}
