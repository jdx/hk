// The benchmark runs the facts tests read: today's published run, frozen
// beside the tests so their figures do not move when a benchmark refresh
// lands, and the live benchmark/results.json as the page and the renderer
// load it. Also the facts variants the reel must survive, which the frames
// preview (showreel-frames.mjs --facts) renders from the same functions.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { factsFromBenchmarks, type RaceKey, type ReelFacts } from "../facts";
import { REPO, SHOWREEL } from "./repo";

/** Parsed JSON, which the tests garble on purpose. */
type Json = any;

/**
 * benchmark/results.json as workflow run 36078397814 published it (hk 2.2.0
 * against lefthook, pre-commit and prek). Parsed afresh on every call, so a
 * test can garble its copy.
 */
export const published = (): Json => JSON.parse(readFileSync(join(SHOWREEL, "test/results-36078397814.json"), "utf8"));

/** The live benchmark/results.json through the loaders' own check (benchmarks.data.ts). */
export function live(): Json {
  try {
    const run = JSON.parse(readFileSync(join(REPO, "benchmark/results.json"), "utf8"));
    if (run.schema !== 2 || run.passed !== true) return null;
    const allCorrect = run.scenarios.every((s: Json) =>
      Object.values(s.results).every((r: Json) => r.correct.passed === r.correct.total),
    );
    return allCorrect ? run : null;
  } catch {
    return null;
  }
}

/** One scenario of a parsed run, by key, for a test to garble. */
export function scenario(run: Json, key: string): Json {
  const found = run.scenarios.find((s: Json) => s.key === key);
  assert.ok(found, `no ${key} scenario`);
  return found;
}

/** One tool's stats in one scenario of a parsed run, for a test to garble. */
export function cell(run: Json, key: string, tool: string): Json {
  const found = scenario(run, key).results[tool];
  assert.ok(found, `no ${key}/${tool} cell`);
  return found;
}

/**
 * Make hk and every other tool level in scenario `key`, as the page would
 * call them: hk's spread widened past its gap to the fastest rival. The
 * race's claim is then withheld, and only that one.
 */
export function unseparate(run: Json, key: RaceKey): Json {
  const s = scenario(run, key);
  const hk = s.results.hk;
  const rival = Math.min(...Object.entries(s.results).filter(([k]) => k !== "hk").map(([, r]: [string, Json]) => r.median));
  hk.max = Math.max(hk.max, rival + (rival - hk.median));
  return run;
}

/** The facts today's run gives: both races, the storyboard's figures. */
export function today(): ReelFacts {
  const facts = factsFromBenchmarks(published());
  assert.ok(facts, "today's run gives no facts");
  return facts;
}

/**
 * Today's run with exactly one claim (F1): Check every file made level with
 * the fastest other tool, so only Fix every file races.
 */
export function oneClaim(): ReelFacts {
  const facts = factsFromBenchmarks(unseparate(published(), "check-all"));
  assert.ok(facts?.fixAll && !facts.checkAll, "the one-claim run does not give exactly Fix every file");
  return facts;
}

/** Facts that back no claim: a sound run in which hk won nothing (F0, with a workload). */
export function noClaim(): ReelFacts {
  return { ...today(), fixAll: null, checkAll: null };
}

/** The facts variants the reel is reviewed and tested under. */
export type Variant = "both" | "one" | "none" | "live";

/** The facts for `variant`: today's run, one claim, no facts at all, or the live results.json. */
export function factsFor(variant: Variant): ReelFacts | null {
  switch (variant) {
    case "both":
      return today();
    case "one":
      return oneClaim();
    case "none":
      return null;
    case "live":
      return factsFromBenchmarks(live());
  }
}

/**
 * The numbers in a text, "6", "157" and "1.3", but not the 3 in "S3" or a
 * version's parts.
 */
export const numbers = (text: string): string[] => text.match(/(?<![\w.])\d+(?:\.\d+)?(?!\w|\.\d)/g) ?? [];
