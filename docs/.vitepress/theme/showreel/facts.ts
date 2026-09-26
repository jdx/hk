// The published benchmark run (benchmark/results.json), reduced to the claims
// the reel and the landing page make from it. Each claim is null unless the
// run backs it, and every scene and chapter text has a line that states no
// number for that case, so a failed, malformed or inconclusive run leaves the
// figures out instead of drawing one the benchmarks page would not stand
// behind. The rules are the benchmarks page's own (benchmarks.data.ts and
// BenchmarkResults.vue) plus the reel's: a race is drawn only when hk is
// clearly ahead of the fastest other tool.
//
// The commit scenario (`fix-staged`) is deliberately not representable: the
// reel shows only whole-repository runs and sends viewers to the page for
// the rest.

/** One tool's timings in one scenario: what the page's `separated` reads. */
export interface Stats {
  median: number;
  min: number;
  max: number;
}

/** A bar in a race, seconds. */
export interface Row extends Stats {
  /** The subject's key in results.json, "hk" or "prek". */
  key: string;
  /** subjects[key].label */
  label: string;
  /** How it ran: stats.mode, else subjects[key].mode, e.g. "parallel: true". */
  mode: string;
  /** The median as the page prints it: fmt(median), "10.6 s". */
  shown: string;
}

export type RaceKey = "fix-all" | "check-all";

/** One scenario the reel races: hk against every other tool. */
export interface Race {
  /** Looked up by key, never by index. */
  key: RaceKey;
  /** scenarios[].title, "Fix every file". */
  title: string;
  /** scenarios[].summary, drawn verbatim as a detail line. */
  summary: string;
  /** The chart's full scale: the largest max over its rows, seconds. */
  axis: number;
  /** hk first, then the other tools by median, fastest first. */
  rows: Row[];
  /** hk against the fastest other tool: `ratio` = (rival.median / hk.median).toFixed(1). */
  claim: { ratio: string; rival: Row };
}

export interface ReelFacts {
  /** "6,157" files, 10 fixers, 8 CPUs: the run's workload and machine. */
  workload: { files: string; fixers: number; cpus: number };
  fixAll: Race | null;
  checkAll: Race | null;
}

/** A duration as the benchmarks page prints it (BenchmarkResults.vue). */
export const fmt = (s: number): string =>
  s >= 10 ? `${s.toFixed(1)} s` : s >= 1 ? `${s.toFixed(2)} s` : `${Math.round(s * 1000)} ms`;

/**
 * Two tools only differ when the gap between their medians is larger than
 * either one's own spread across samples; otherwise the page calls them
 * level (BenchmarkResults.vue).
 */
export const separated = (a: Stats, b: Stats): boolean =>
  Math.abs(a.median - b.median) > Math.max(a.max - a.min, b.max - b.min);

/** A caption line holds at most this many characters (type.ts, captions.test.ts). */
const CAPTION_CHARS = 36;

/** The caption line a race's claim is stated in. */
export const claimLine = (r: Pick<Race, "title" | "claim">): string => `${r.title}: ${r.claim.ratio}× faster`;

// Structural subset of docs/.vitepress/benchmarks.data.ts. Everything is
// checked as it is read, because the tests garble it on purpose.
type Json = Record<string, unknown>;

const isObject = (v: unknown): v is Json => typeof v === "object" && v !== null && !Array.isArray(v);
/** A positive, finite number of seconds. */
const positive = (n: unknown): n is number => typeof n === "number" && Number.isFinite(n) && n > 0;
/** A whole, positive count. */
const count = (n: unknown): n is number => typeof n === "number" && Number.isSafeInteger(n) && n > 0;
/** Copy fit to print on one line: plain text, no control characters, not too long. */
const printable = (v: unknown, max: number): v is string =>
  typeof v === "string" && v.trim() === v && v.length > 0 && v.length <= max && !/[\p{Cc}\p{Co}\p{Cn}<>`]/u.test(v);

/** Every tool in every scenario produced exactly the right files. */
function allCorrect(scenarios: unknown[]): boolean {
  return scenarios.every(
    (s) =>
      isObject(s) &&
      isObject(s.results) &&
      Object.values(s.results).every(
        (r) =>
          isObject(r) &&
          isObject(r.correct) &&
          typeof r.correct.passed === "number" &&
          r.correct.passed === r.correct.total,
      ),
  );
}

/** One subject's row, or null if any of it is unfit to draw. */
function row(key: string, stats: unknown, subjects: Json): Row | null {
  if (!isObject(stats)) return null;
  const subject = subjects[key];
  if (!isObject(subject)) return null;
  const { median, min, max } = stats;
  if (!positive(median) || !positive(min) || !positive(max)) return null;
  // A whisker that does not hold its median is garbled.
  if (!(min <= median && median <= max)) return null;
  const label = subject.label;
  const mode = stats.mode ?? subject.mode;
  if (!printable(label, 24) || !printable(mode, 40)) return null;
  return { key, label, mode, median, min, max, shown: fmt(median) };
}

/** The race in scenario `key`, or null unless every rule holds. */
function race(key: RaceKey, scenarios: unknown[], subjects: Json): Race | null {
  const s = scenarios.find((x) => isObject(x) && x.key === key);
  if (!isObject(s) || !isObject(s.results)) return null;
  const { title, summary } = s;
  if (!printable(title, 30) || !printable(summary, 100)) return null;
  const entries = Object.entries(s.results);
  const rows = entries.map(([k, stats]) => row(k, stats, subjects));
  // Every drawn stat must be sound, not just hk's and the fastest rival's.
  if (rows.some((r) => r === null)) return null;
  const drawn = rows as Row[];
  const hk = drawn.find((r) => r.key === "hk");
  const rivals = drawn.filter((r) => r.key !== "hk").sort((a, b) => a.median - b.median);
  if (!hk || !rivals.length) return null;
  const rival = rivals[0];
  if (!separated(hk, rival) || !(rival.median / hk.median > 1)) return null;
  const claim = { ratio: (rival.median / hk.median).toFixed(1), rival };
  // A ratio that rounds to 1.0 claims nothing.
  if (claim.ratio === "1.0") return null;
  if (claimLine({ title, claim }).length > CAPTION_CHARS) return null;
  return { key, title, summary, axis: Math.max(...drawn.map((r) => r.max)), rows: [hk, ...rivals], claim };
}

/**
 * The claims in a published run, or null for a run that is missing, not
 * parsable, from another schema, failed, reported problems, or has a tool
 * that did not produce the right files: the benchmarks page shows nothing
 * for such a run either.
 */
export function factsFromBenchmarks(results: unknown): ReelFacts | null {
  if (!isObject(results)) return null;
  if (results.schema !== 2 || results.passed !== true) return null;
  const { problems, scenarios, subjects, workload, machine } = results;
  if (problems !== undefined && !(Array.isArray(problems) && problems.length === 0)) return null;
  if (!Array.isArray(scenarios) || !isObject(subjects)) return null;
  if (!allCorrect(scenarios)) return null;
  // The workload line states what every race measured, so a run without one
  // states nothing.
  if (!isObject(workload) || !isObject(machine)) return null;
  if (!count(workload.files) || !Array.isArray(workload.fixers) || !workload.fixers.length || !count(machine.cpus)) return null;
  return {
    workload: { files: workload.files.toLocaleString("en-US"), fixers: workload.fixers.length, cpus: machine.cpus },
    fixAll: race("fix-all", scenarios, subjects),
    checkAll: race("check-all", scenarios, subjects),
  };
}

/** The races the facts back, in the order the reel runs them. */
export const races = (f: ReelFacts | null): Race[] => [f?.fixAll ?? null, f?.checkAll ?? null].filter((r): r is Race => r !== null);
