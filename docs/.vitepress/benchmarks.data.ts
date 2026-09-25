import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// Written by benchmark/report.py; see benchmark/README.md.
const resultsPath = fileURLToPath(
  new URL("../../benchmark/results.json", import.meta.url),
);

export interface Stats {
  mean: number;
  median: number;
  stddev: number;
  min: number;
  max: number;
  runs: number;
  correct: { passed: number; total: number };
}

export interface Subject {
  tool: string;
  label: string;
  mode: string;
}

export interface Scenario {
  key: string;
  title: string;
  summary: string;
  results: Record<string, Stats>;
}

export interface BenchmarkResults {
  schema: 2;
  generated: string;
  passed: boolean;
  commit: string;
  workflow_run: string | null;
  seed: string | null;
  machine: {
    runner: string;
    os: string | null;
    kernel: string | null;
    arch: string | null;
    cpu: string | null;
    cpus: number | null;
    memory_gb: number | null;
  };
  versions: Record<string, string | null>;
  workload: {
    files: number;
    dirty_files: number;
    staged_files: number | null;
    fixers: string[];
  };
  subjects: Record<string, Subject>;
  scenarios: Scenario[];
}

declare const data: BenchmarkResults | null;
export { data };

export default {
  watch: [resultsPath],
  load(): BenchmarkResults | null {
    if (!existsSync(resultsPath)) return null;
    const results = JSON.parse(readFileSync(resultsPath, "utf8"));
    // An unverified or failed run must never render as if it were sound.
    if (results.schema !== 2 || results.passed !== true) return null;
    // The page presents every tool as correct, so reject a run in which one wasn't.
    const allCorrect = (results as BenchmarkResults).scenarios.every((s) =>
      Object.values(s.results).every((r) => r.correct.passed === r.correct.total),
    );
    if (!allCorrect) return null;
    return results;
  },
};
