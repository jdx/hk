// The benchmark runs the facts tests read: today's published run and the
// one before it, frozen beside the tests so their figures do not move when a
// benchmark refresh lands, and the live benchmark/results.json as the page
// and the renderer load it. Also the facts variants the reel must survive,
// which the frames preview (showreel-frames.mjs --facts) and draft renders
// (showreel-video.mjs --facts) build from the same functions.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { factsFromBenchmarks, type RaceKey, type ReelFacts } from "../facts";
import { REPO, SHOWREEL } from "./repo";

/** Parsed JSON, which the tests garble on purpose. */
type Json = any;

/** A frozen run, parsed afresh on every call, so a test can garble its copy. */
const frozen = (run: string): Json => JSON.parse(readFileSync(join(SHOWREEL, `test/results-${run}.json`), "utf8"));

/**
 * benchmark/results.json as workflow run 36268162842 published it (hk 2.3.0
 * against lefthook, pre-commit and prek): hk leads in every scenario, by the
 * most in the commit (1.6× lefthook), so the commit races.
 */
export const published = (): Json => frozen("36268162842");

/**
 * The run before it, 36078397814 (hk 2.2.0): lefthook was faster in the
 * commit, and hk leads Check every file (1.8× lefthook) by more than Fix
 * every file (1.3× prek), so Check every file races.
 */
export const previous = (): Json => frozen("36078397814");

/** The live benchmark/results.json through the loaders' own check (benchmarks.data.ts). */
export function live(): Json {
  try {
    const run = JSON.parse(readFileSync(join(REPO, "benchmark/results.json"), "utf8"));
    if (run.schema !== 3 || run.passed !== true) return null;
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
 * call them: hk's spread widened past its gap to the fastest rival. That
 * scenario is then no candidate, and only that one.
 */
export function unseparate(run: Json, key: RaceKey): Json {
  const s = scenario(run, key);
  const hk = s.results.hk;
  const rival = Math.min(...Object.entries(s.results).filter(([k]) => k !== "hk").map(([, r]: [string, Json]) => r.median));
  hk.max = Math.max(hk.max, rival + (rival - hk.median));
  return run;
}

/** `run` with only scenario `key` left in it: its facts race that scenario if it is a candidate. */
export function only(run: Json, key: RaceKey): Json {
  scenario(run, key);
  run.scenarios = run.scenarios.filter((s: Json) => s.key === key);
  return run;
}

/** The facts today's run gives: the commit races, with the frozen run's figures. */
export function today(): ReelFacts {
  const facts = factsFromBenchmarks(published());
  assert.ok(facts?.best?.key === "fix-staged", "today's run does not race the commit");
  return facts;
}

/**
 * The facts the previous run gives, real data for another race: Check every
 * file. It draws that run's own summary line for it ("…, so parallel is safe
 * for every tool."), which today's run has since shortened.
 */
export function alt(): ReelFacts {
  const facts = factsFromBenchmarks(previous());
  assert.ok(facts?.best?.key === "check-all", "the previous run does not race Check every file");
  return facts;
}

/** Facts that back no claim: a sound run in which hk won nothing (F0, with a workload). */
export function noClaim(): ReelFacts {
  return { ...today(), best: null };
}

/** The facts variants the reel is reviewed and tested under. */
export const VARIANTS = ["today", "alt", "none", "live"] as const;
export type Variant = (typeof VARIANTS)[number];

/**
 * The facts for `variant`: today's run (the commit), the previous run
 * (Check every file), no facts at all, or the live results.json.
 */
export function factsFor(variant: Variant): ReelFacts | null {
  switch (variant) {
    case "today":
      return today();
    case "alt":
      return alt();
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
