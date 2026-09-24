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
  correct: { passed: number; trials: number; max_wrong_files: number };
}

export interface Subject {
  tool: string;
  label: string;
  mode: string;
  safe: boolean;
}

export interface Scenario {
  key: string;
  title: string;
  summary: string;
  results: Record<string, Stats>;
}

export interface BenchmarkResults {
  schema: 1;
  generated: string;
  passed: boolean;
  commit: string;
  workflow_run: string | null;
  machine: {
    runner: string;
    os: string;
    arch: string;
    cpu: string;
    cpus: number;
    memory_gb: number;
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
    if (results.schema !== 1 || results.passed !== true) return null;
    return results;
  },
};
