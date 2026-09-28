// The reel's figures come from the published benchmark run through facts.ts,
// which races the one scenario hk leads by the most. Today's run, frozen in
// test/results-36268162842.json, races the commit (1.6× lefthook, ahead of
// Check every file's 1.3× and Fix every file's 1.2×); the run before it,
// frozen in test/results-36078397814.json, races Check every file (1.8×
// lefthook, ahead of Fix every file's 1.3×; lefthook won the commit). A
// scenario that is garbled, too close to call or not won is never chosen;
// a run that is missing, failed or malformed gives no facts; and a run that
// backs no race states no number in any caption.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { claimLine, factsFromBenchmarks, fmt, lead, MODE_CHARS, type Race, RACE_KEYS, type RaceKey, type ReelFacts, races, separated, type Stats } from "../facts";
import { scenes } from "../scenes";
import type { SectionId } from "../timeline";
import { plain } from "../type";
import { alt, cell, factsFor, live, noClaim, only, previous, published, scenario, today, unseparate } from "./published";
import { REPO } from "./repo";

/** Parsed JSON, which the tests garble on purpose. */
type Run = ReturnType<typeof published>;

/** A row as the chart reads it: key, shown median, mode. */
const readout = (r: Race | null) => r?.rows.map((x) => `${x.key} ${x.shown} (${x.mode})`) ?? null;

/** Each row's published numbers, not the rounded readout, in the race's row order. */
const numbersOf = (r: Race) => r.rows.map(({ key, median, min, max }) => ({ key, median, min, max }));
const publishedNumbers = (run: Run, key: RaceKey, tools: string[]) =>
  tools.map((tool) => {
    const { median, min, max } = cell(run, key, tool);
    return { key: tool, median, min, max };
  });

/** Scenario `key` of a run as a candidate: its race, or null if the run does not back it. */
const candidate = (run: Run, key: RaceKey): Race | null => factsFromBenchmarks(only(run, key))?.best ?? null;

/** A number to four places, for comparing leads. */
const places4 = (n: number | undefined) => (n === undefined ? n : Math.round(n * 1e4) / 1e4);

test("today's run races the commit, the scenario hk leads by the most", () => {
  const f = today();
  assert.deepEqual(Object.keys(f), ["workload", "best"]);
  assert.deepEqual(f.workload, { files: "6,157", fixers: 10, cpus: 8 });
  assert.deepEqual(races(f), [f.best]);

  // Every scenario is a candidate today; the commit leads by the most.
  const all = RACE_KEYS.map((key) => candidate(published(), key));
  assert.deepEqual(all.map((r) => r?.claim.ratio), ["1.6", "1.2", "1.3"]);
  assert.deepEqual(all.map((r) => places4(r ? lead(r) : undefined)), [1.5657, 1.2487, 1.2847]);
  assert.deepEqual(f.best, all[0]);

  const r = f.best;
  assert.ok(r);
  assert.equal(r.key, "fix-staged");
  assert.equal(r.title, "Commit");
  assert.equal(r.summary, "About 60 staged files with defects, fixed by each tool's pre-commit hook.");
  assert.deepEqual(readout(r), [
    "hk 907 ms (parallel, file locks)",
    "lefthook 1.42 s (sequential)",
    "prek 1.77 s (sequential hooks, batched files)",
    "pre-commit 2.44 s (sequential hooks, batched files)",
  ]);
  assert.equal(r.axis, 2.523);
  assert.equal(r.claim.ratio, "1.6");
  assert.equal(r.claim.rival.key, "lefthook");
  assert.equal(claimLine(r), "Commit: 1.6× faster");
  assert.deepEqual(numbersOf(r), publishedNumbers(published(), "fix-staged", ["hk", "lefthook", "prek", "pre-commit"]));
});

test("the previous run races Check every file: lefthook won the commit, and hk leads Check every file by more than Fix every file", () => {
  const run = previous();
  const staged = scenario(run, "fix-staged").results;
  assert.ok(staged.lefthook.median < staged.hk.median, "lefthook was faster in the previous run's commit");
  assert.equal(candidate(previous(), "fix-staged"), null);
  const fixAll = candidate(previous(), "fix-all");
  assert.ok(fixAll);
  assert.equal(fixAll.claim.ratio, "1.3");

  const f = factsFromBenchmarks(run);
  assert.ok(f);
  assert.deepEqual(f.workload, { files: "6,157", fixers: 10, cpus: 8 });
  const r = f.best;
  assert.ok(r);
  assert.deepEqual(f, alt());
  assert.equal(r.key, "check-all");
  assert.equal(r.title, "Check every file");
  assert.equal(r.summary, "Every file is already clean and nothing writes, so parallel is safe for every tool.");
  // lefthook's mode here is the scenario's own, not its subject's.
  assert.deepEqual(readout(r), [
    "hk 3.82 s (parallel, file locks)",
    "lefthook 6.77 s (parallel: true)",
    "prek 7.55 s (sequential hooks, batched files)",
    "pre-commit 8.19 s (sequential hooks, batched files)",
  ]);
  assert.equal(r.axis, 12.008);
  assert.equal(r.claim.ratio, "1.8");
  assert.equal(r.claim.rival.key, "lefthook");
  assert.equal(claimLine(r), "Check every file: 1.8× faster");
  assert.deepEqual(numbersOf(r), publishedNumbers(run, "check-all", ["hk", "lefthook", "prek", "pre-commit"]));
  assert.deepEqual([places4(lead(r)), places4(lead(fixAll))], [1.7727, 1.293]);
  assert.deepEqual(races(f), [r]);
});

/**
 * Today's run with every scenario given the commit's timings, scaled by 1,
 * 2 and 4 in the order the scenarios are tried: every lead is exactly the
 * same, though no two scenarios' times are.
 */
function tied(): Run {
  const run = published();
  const commit = scenario(run, "fix-staged").results;
  RACE_KEYS.forEach((key, i) => {
    const k = 2 ** i;
    scenario(run, key).results = Object.fromEntries(
      Object.entries(commit).map(([tool, r]: [string, Run]) => [tool, { ...r, median: r.median * k, min: r.min * k, max: r.max * k }]),
    );
  });
  return run;
}

test("an exact tie goes to the first in the order: the commit, Fix every file, Check every file", () => {
  const leads = RACE_KEYS.map((key) => {
    const r = candidate(tied(), key);
    assert.ok(r, `${key} is a candidate`);
    return lead(r);
  });
  assert.equal(new Set(leads).size, 1, `the leads are not exactly tied: ${leads}`);
  assert.equal(factsFromBenchmarks(tied())?.best?.key, "fix-staged");
  const later = tied();
  unseparate(later, "fix-staged");
  assert.equal(factsFromBenchmarks(later)?.best?.key, "fix-all");
  // And whichever order the run lists them in.
  const reversed = tied();
  reversed.scenarios.reverse();
  assert.equal(factsFromBenchmarks(reversed)?.best?.key, "fix-staged");
});

test("leads are compared unrounded: of two that both read 1.2×, the larger races", () => {
  const fixAll = candidate(published(), "fix-all");
  assert.ok(fixAll);
  assert.equal(fixAll.claim.ratio, "1.2");
  // Today's Fix every file leads by 1.2487. Without the commit, Check every
  // file races only when its lead is larger, even by a hair.
  for (const [ratio, winner] of [
    [1.2495, "check-all"],
    [1.248, "fix-all"],
  ] as const) {
    const run = published();
    unseparate(run, "fix-staged");
    const hk = cell(run, "check-all", "hk");
    hk.median = cell(run, "check-all", "lefthook").median / ratio;
    const f = factsFromBenchmarks(run);
    assert.equal(candidate(run, "check-all")?.claim.ratio, "1.2", `${ratio}: Check every file still claims 1.2×`);
    assert.equal(f?.best?.key, winner, `${ratio} against ${lead(fixAll)}`);
  }
});

test("a scenario that is too close to call (however large its lead), not won, or won by less than 1.05× is never chosen", () => {
  // Level with lefthook in the commit, hk still leads it by 1.57, and Check every file races instead.
  const level = published();
  unseparate(level, "fix-staged");
  assert.ok(cell(level, "fix-staged", "lefthook").median / cell(level, "fix-staged", "hk").median > 1.5);
  assert.equal(factsFromBenchmarks(level)?.best?.key, "check-all");
  assert.equal(candidate(level, "fix-staged"), null);
  // Slower than a rival, though clearly: not a candidate even on its own,
  // where nothing else could outrank it, so the next one races.
  const lost = published();
  const hk = cell(lost, "fix-staged", "hk");
  Object.assign(hk, { median: hk.median * 10, min: hk.min * 10, max: hk.max * 10 });
  assert.ok(separated(hk, cell(lost, "fix-staged", "lefthook")));
  assert.equal(factsFromBenchmarks(lost)?.best?.key, "check-all");
  assert.equal(candidate(lost, "fix-staged"), null);
  // Clearly ahead, but by a ratio that rounds to 1.0, which claims nothing.
  const close = only(published(), "check-all");
  const lefthook = cell(close, "check-all", "lefthook");
  Object.assign(lefthook, { min: lefthook.median - 0.01, max: lefthook.median + 0.01 });
  Object.assign(cell(close, "check-all", "hk"), { median: lefthook.median / 1.04, min: lefthook.median / 1.04 - 0.01, max: lefthook.median / 1.04 + 0.01 });
  assert.ok(separated(cell(close, "check-all", "hk"), lefthook));
  assert.equal(factsFromBenchmarks(close)?.best, null);
});

test("a scenario that does not race leaks none of its figures", () => {
  // Nothing from a scenario's results may reach the facts unless its race is drawn.
  const leaks = (f: ReelFacts, run: Run, key: string) => {
    const json = JSON.stringify(f);
    const s = scenario(run, key);
    assert.ok(!json.includes(`"${key}"`) && !json.includes(`"${s.title}"`) && !json.includes(s.summary), `${key} is in the facts`);
    for (const r of Object.values(s.results) as Stats[]) {
      assert.ok(!json.includes(String(r.median)), `${key}'s ${r.median} leaked into the facts`);
      assert.ok(!json.includes(fmt(r.median)), `${key}'s ${fmt(r.median)} leaked into the facts`);
    }
  };
  leaks(today(), published(), "fix-all");
  leaks(today(), published(), "check-all");
  leaks(alt(), previous(), "fix-staged");
  leaks(alt(), previous(), "fix-all");
});

/** A function the benchmarks page defines, lifted out of BenchmarkResults.vue and run as JavaScript. */
function pageFunction(name: string): (...args: never[]) => unknown {
  const vue = readFileSync(join(REPO, "docs/.vitepress/theme/BenchmarkResults.vue"), "utf8");
  const m = vue.match(new RegExp(`const ${name} = \\(([^)]*)\\) =>([\\s\\S]*?);\\n`));
  assert.ok(m, `BenchmarkResults.vue defines no ${name}`);
  const params = m[1].replace(/:\s*\w+/g, "");
  return new Function(`return (${params}) => ${m[2]};`)();
}

test("fmt() and separated() are the benchmarks page's own", () => {
  const pageFmt = pageFunction("fmt") as (s: number) => string;
  for (const s of [0.0004, 0.2, 0.9067, 0.9994, 0.9996, 1, 1.4196, 3.8165, 9.4324, 9.994, 9.996, 10, 10.6263, 24.8556, 123.45]) {
    assert.equal(fmt(s), pageFmt(s), `fmt(${s})`);
  }
  assert.equal(fmt(10.6263), "10.6 s");
  assert.equal(fmt(9.4324), "9.43 s");
  assert.equal(fmt(3.8165), "3.82 s");
  assert.equal(fmt(0.9067), "907 ms");
  assert.equal(fmt(0.5), "500 ms");

  const pageSeparated = pageFunction("separated") as (a: Stats, b: Stats) => boolean;
  const s = (median: number, min: number, max: number): Stats => ({ median, min, max });
  const pairs: [Stats, Stats][] = [
    [s(2, 1, 3), s(4, 3, 5)],
    [s(2, 1, 3), s(4.01, 3.5, 5)],
    [s(4.01, 3.5, 5), s(2, 1, 3)],
    [s(3.8165, 3.7489, 3.873), s(6.7654, 6.6835, 6.8072)],
    [s(0.9067, 0.8543, 1.1777), s(1.4196, 1.3852, 1.4529)],
    [s(9.4324, 9.318, 9.5742), s(11.7781, 11.1946, 12.2389)],
    [s(8.188, 6.0207, 12.008), s(7.5546, 5.3041, 11.3261)],
  ];
  for (const [a, b] of pairs) assert.equal(separated(a, b), pageSeparated(a, b), JSON.stringify([a, b]));
  // A gap as wide as the spread is still level; a hair wider is not.
  assert.ok(!separated(s(2, 1, 3), s(4, 3, 5)));
  assert.ok(separated(s(2, 1, 3), s(4.01, 3.5, 5)));
});

test("a missing, failed or unreadable run gives no facts", () => {
  const unreadable: unknown[] = [
    null,
    undefined,
    {},
    "results",
    42,
    [],
    { schema: 2, passed: true },
    { schema: 2, passed: true, scenarios: "fix-all", subjects: {} },
  ];
  for (const data of unreadable) assert.equal(factsFromBenchmarks(data), null, JSON.stringify(data));

  const cases: [string, (run: Run) => void][] = [
    ["another schema", (r) => (r.schema = 1)],
    ["a failed run", (r) => (r.passed = false)],
    ["a run that does not say it passed", (r) => delete r.passed],
    ["a run with problems", (r) => r.problems.push("prek wrote the wrong bytes")],
    ["problems that are not a list", (r) => (r.problems = "none")],
    ["a tool that got a file wrong", (r) => (cell(r, "fix-staged", "lefthook").correct.passed = 19)],
    ["a tool with no correctness count", (r) => delete cell(r, "check-all", "prek").correct],
    ["no subjects", (r) => delete r.subjects],
    ["no workload", (r) => delete r.workload],
    ["no file count", (r) => (r.workload.files = "many")],
    ["no fixers", (r) => (r.workload.fixers = [])],
    ["no CPU count", (r) => (r.machine.cpus = null)],
  ];
  for (const [what, garble] of cases) {
    const run = published();
    garble(run);
    assert.equal(factsFromBenchmarks(run), null, what);
  }
});

const GARBLED: [string, (run: Run, key: RaceKey) => void][] = [
  ["no such scenario", (r, key) => (r.scenarios = r.scenarios.filter((s: { key: string }) => s.key !== key))],
  ["no hk row", (r, key) => delete scenario(r, key).results.hk],
  [
    "no other tool",
    (r, key) => {
      const s = scenario(r, key);
      s.results = { hk: s.results.hk };
    },
  ],
  ["a median that is not a number", (r, key) => (cell(r, key, "hk").median = "fast")],
  ["an infinite max", (r, key) => (cell(r, key, "lefthook").max = Infinity)],
  ["a negative min", (r, key) => (cell(r, key, "pre-commit").min = -1)],
  ["a zero median", (r, key) => (cell(r, key, "prek").median = 0)],
  ["a whisker that misses its median", (r, key) => (cell(r, key, "prek").min = cell(r, key, "prek").max + 1)],
  ["a tool with no subject", (r) => delete r.subjects.prek],
  ["a label that is not plain", (r) => (r.subjects.lefthook.label = "lefthook\n<script>")],
  [
    "a mode that is not text",
    (r, key) => {
      cell(r, key, "lefthook").mode = 7;
    },
  ],
  ["hk level with the fastest other tool", (r, key) => unseparate(r, key)],
  [
    "hk slower than another tool",
    (r, key) => {
      const hk = cell(r, key, "hk");
      Object.assign(hk, { median: hk.median * 10, min: hk.min * 10, max: hk.max * 10 });
    },
  ],
  ["a title too long for the caption", (r, key) => (scenario(r, key).title = "Fix every file in every repo")],
  ["no title", (r, key) => delete scenario(r, key).title],
  ["no summary", (r, key) => (scenario(r, key).summary = "")],
];

/** The race that runs when scenario `key` is garbled out of today's run: the best of the other two. */
const NEXT: Record<RaceKey, RaceKey> = { "fix-staged": "check-all", "fix-all": "fix-staged", "check-all": "fix-staged" };

test("a garbled or inconclusive scenario is no candidate, and the best of the others races", () => {
  const good = today();
  for (const key of RACE_KEYS) {
    const next = candidate(published(), NEXT[key]);
    assert.ok(next);
    for (const [what, garble] of GARBLED) {
      const run = published();
      garble(run, key);
      const f = factsFromBenchmarks(run);
      assert.ok(f, `${key} with ${what}: the run gave no facts`);
      assert.notEqual(f.best?.key, key, `${key} with ${what} still races`);
      assert.deepEqual(f.workload, good.workload, `${key} with ${what} changed the workload`);
      // A subject's label and mode are shared by every scenario, so garbling one reaches them all.
      if (what === "a tool with no subject" || what === "a label that is not plain") assert.equal(f.best, null, `${key} with ${what}`);
      else assert.deepEqual(f.best, next, `${key} with ${what} races ${f.best?.key}, not ${NEXT[key]}`);
    }
  }
});

test("the variants the preview, the drafts and the tests share", () => {
  assert.deepEqual(factsFor("today"), factsFromBenchmarks(published()));
  assert.equal(factsFor("today")?.best?.claim.ratio, "1.6");
  assert.deepEqual(factsFor("alt"), factsFromBenchmarks(previous()));
  assert.equal(factsFor("alt")?.best?.claim.ratio, "1.8");
  assert.equal(factsFor("none"), null);
  assert.deepEqual(factsFor("live"), factsFromBenchmarks(live()));
  const none = noClaim();
  assert.deepEqual(none.workload, today().workload);
  assert.equal(none.best, null);
  assert.deepEqual(races(none), []);
  assert.deepEqual(races(null), []);
});

test("the reel reads every run the benchmarks page shows", () => {
  // Otherwise a schema bump that updates only the page's loader leaves the
  // landing page without its race, and the test below passes on no facts.
  const run = live();
  if (run) assert.ok(factsFromBenchmarks(run), `the page shows schema ${run.schema} results, which the reel rejects`);
});

test("every mode in the live results.json fits under its bar", () => {
  // A longer mode would take its whole scenario out of the race.
  const run = live();
  if (!run) return;
  for (const s of run.scenarios.filter((s: { key: RaceKey }) => RACE_KEYS.includes(s.key))) {
    for (const [tool, stats] of Object.entries(s.results) as [string, { mode?: string }][]) {
      const mode = stats.mode ?? run.subjects[tool].mode;
      assert.ok(mode.length <= MODE_CHARS, `${s.key}/${tool} mode "${mode}" is over ${MODE_CHARS} characters`);
    }
  }
});

test("the live results.json gives a race that holds together and leads by the most, or none", () => {
  // A refresh may publish a run too close to call, which leaves the race
  // out; it must never give one that contradicts itself.
  const run = live();
  const r = factsFromBenchmarks(run)?.best;
  if (!r) return;
  const [hk, ...rivals] = r.rows;
  assert.equal(hk.key, "hk");
  assert.ok(rivals.length > 0);
  assert.equal(r.claim.rival, rivals[0]);
  assert.ok(separated(hk, rivals[0]));
  assert.ok(Number(r.claim.ratio) > 1);
  assert.ok(claimLine(r).length <= 36);
  for (let i = 1; i < rivals.length; i++) assert.ok(rivals[i - 1].median <= rivals[i].median);
  for (const row of r.rows) assert.ok(row.max <= r.axis);
  for (const key of RACE_KEYS) {
    if (!run.scenarios.some((s: { key: string }) => s.key === key)) continue;
    const other = candidate(live(), key);
    if (!other) continue;
    const before: boolean = RACE_KEYS.indexOf(key) < RACE_KEYS.indexOf(r.key);
    assert.ok(before ? lead(other) < lead(r) : lead(other) <= lead(r), `${key} leads by ${lead(other)}, more than ${r.key}'s ${lead(r)}`);
  }
});

/** A scene's caption lines under `facts`, as plain text. */
function lines(id: SectionId, facts: ReelFacts | null): string[] {
  const scene = scenes.find((s) => s.id === id);
  return (scene?.captions?.(facts) ?? []).flatMap((c) => c.lines.map((l) => plain(l.text)));
}

test("no caption shows a number the facts do not back", () => {
  const every = scenes.map((s) => s.id);
  const others = every.filter((id) => id !== "race");
  // Only the race states numbers, and only its claim's ratio.
  for (const id of others) for (const line of lines(id, today())) assert.doesNotMatch(line, /\d/, `${id}: "${line}"`);
  const variants: [string, ReelFacts | null, string[]][] = [
    ["no facts", null, []],
    ["no claim", noClaim(), []],
    ["today's run: the commit", today(), ["1.6"]],
    ["the previous run: Check every file", alt(), ["1.8"]],
    // Every other candidate either run has, raced alone.
    ["today's Fix every file", factsFromBenchmarks(only(published(), "fix-all")), ["1.2"]],
    ["today's Check every file", factsFromBenchmarks(only(published(), "check-all")), ["1.3"]],
    ["the previous run's Fix every file", factsFromBenchmarks(only(previous(), "fix-all")), ["1.3"]],
  ];
  for (const [what, facts, allowed] of variants) {
    for (const id of every) {
      for (const line of lines(id, facts)) {
        const found = line.match(/\d+(?:\.\d+)?/g) ?? [];
        for (const n of found) assert.ok(allowed.includes(n), `${id} with ${what}: "${line}"`);
      }
    }
    const said = every.flatMap((id) => lines(id, facts)).join(" ");
    for (const n of allowed) assert.ok(said.includes(`${n}×`), `${what}: the ${n}× claim is missing`);
  }
});
