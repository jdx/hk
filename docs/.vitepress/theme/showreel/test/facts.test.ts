// The reel's figures come from the published benchmark run through facts.ts.
// Today's run, frozen in test/results-36078397814.json, gives fixed figures.
// A run that is missing, failed, malformed or too close to call gives none,
// or withholds just the race it cannot back, and then no caption shows a
// number. The commit scenario is never representable.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { claimLine, factsFromBenchmarks, fmt, type Race, type RaceKey, type ReelFacts, separated, type Stats } from "../facts";
import { scenes } from "../scenes";
import type { SectionId } from "../timeline";
import { plain } from "../type";
import { cell, live, noClaim, oneClaim, published, scenario, today, unseparate } from "./published";
import { REPO } from "./repo";

/** A row as the chart reads it: key, shown median, mode. */
const readout = (r: Race | null) => r?.rows.map((x) => `${x.key} ${x.shown} (${x.mode})`) ?? null;

test("today's run gives its published figures", () => {
  const f = today();
  assert.deepEqual(Object.keys(f), ["workload", "fixAll", "checkAll"]);
  assert.deepEqual(f.workload, { files: "6,157", fixers: 10, cpus: 8 });

  assert.ok(f.fixAll);
  assert.equal(f.fixAll.key, "fix-all");
  assert.equal(f.fixAll.title, "Fix every file");
  assert.equal(f.fixAll.summary, "A quarter of the files need two or three fixers each to write them.");
  assert.deepEqual(readout(f.fixAll), [
    "hk 10.6 s (parallel, file locks)",
    "prek 13.7 s (sequential hooks, batched files)",
    "pre-commit 14.6 s (sequential hooks, batched files)",
    "lefthook 24.9 s (sequential)",
  ]);
  assert.equal(f.fixAll.axis, 28.2999);
  assert.equal(f.fixAll.claim.ratio, "1.3");
  assert.equal(f.fixAll.claim.rival.key, "prek");
  assert.equal(claimLine(f.fixAll), "Fix every file: 1.3× faster");

  assert.ok(f.checkAll);
  assert.equal(f.checkAll.key, "check-all");
  assert.equal(f.checkAll.title, "Check every file");
  assert.deepEqual(readout(f.checkAll), [
    "hk 3.82 s (parallel, file locks)",
    "lefthook 6.77 s (parallel: true)",
    "prek 7.55 s (sequential hooks, batched files)",
    "pre-commit 8.19 s (sequential hooks, batched files)",
  ]);
  assert.equal(f.checkAll.axis, 12.008);
  assert.equal(f.checkAll.claim.ratio, "1.8");
  assert.equal(f.checkAll.claim.rival.key, "lefthook");
  assert.equal(claimLine(f.checkAll), "Check every file: 1.8× faster");
  // Each row keeps the published numbers, not the rounded readout.
  assert.deepEqual(
    f.fixAll.rows.map(({ key, median, min, max }) => ({ key, median, min, max })),
    ["hk", "prek", "pre-commit", "lefthook"].map((key) => {
      const { median, min, max } = cell(published(), "fix-all", key);
      return { key, median, min, max };
    }),
  );
});

test("the commit scenario is never representable", () => {
  const f = today();
  const json = JSON.stringify(f);
  const staged = scenario(published(), "fix-staged");
  assert.doesNotMatch(json, /fix-staged|Commit/);
  for (const r of Object.values(staged.results) as Stats[]) {
    assert.ok(!json.includes(String(r.median)), `fix-staged's ${r.median} leaked into the facts`);
    assert.ok(!json.includes(fmt(r.median)), `fix-staged's ${fmt(r.median)} leaked into the facts`);
  }
  // Nor does it stand in for a race that is missing.
  const run = published();
  run.scenarios = run.scenarios.filter((s: { key: string }) => s.key === "fix-staged");
  const only = factsFromBenchmarks(run);
  assert.ok(only);
  assert.equal(only.fixAll, null);
  assert.equal(only.checkAll, null);
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
  for (const s of [0.0004, 0.2, 0.9994, 0.9996, 1, 3.8165, 9.994, 9.996, 10, 10.6263, 24.8556, 123.45]) {
    assert.equal(fmt(s), pageFmt(s), `fmt(${s})`);
  }
  assert.equal(fmt(10.6263), "10.6 s");
  assert.equal(fmt(3.8165), "3.82 s");
  assert.equal(fmt(0.5), "500 ms");

  const pageSeparated = pageFunction("separated") as (a: Stats, b: Stats) => boolean;
  const s = (median: number, min: number, max: number): Stats => ({ median, min, max });
  const pairs: [Stats, Stats][] = [
    [s(2, 1, 3), s(4, 3, 5)],
    [s(2, 1, 3), s(4.01, 3.5, 5)],
    [s(4.01, 3.5, 5), s(2, 1, 3)],
    [s(3.8165, 3.7489, 3.873), s(6.7654, 6.6835, 6.8072)],
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

  const cases: [string, (run: ReturnType<typeof published>) => void][] = [
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

type Claim = "fixAll" | "checkAll";
const KEYS: Record<Claim, RaceKey> = { fixAll: "fix-all", checkAll: "check-all" };

const GARBLED: [string, (run: ReturnType<typeof published>, key: RaceKey) => void][] = [
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

test("a garbled or inconclusive race is withheld, and only that one", () => {
  const good = today();
  for (const claim of ["fixAll", "checkAll"] as const) {
    const other: Claim = claim === "fixAll" ? "checkAll" : "fixAll";
    for (const [what, garble] of GARBLED) {
      const run = published();
      garble(run, KEYS[claim]);
      const f = factsFromBenchmarks(run);
      assert.ok(f, `${claim} with ${what}: every claim went`);
      assert.equal(f[claim], null, `${claim} with ${what} still claims ${JSON.stringify(f[claim])}`);
      assert.deepEqual(f.workload, good.workload, `${claim} with ${what} changed the workload`);
      // A subject's label and mode are shared by both races, so garbling one reaches both.
      if (what !== "a tool with no subject" && what !== "a label that is not plain") {
        assert.deepEqual(f[other], good[other], `${claim} with ${what} changed ${other}`);
      }
    }
  }
});

test("the one-claim and no-claim variants are what the preview and the tests share", () => {
  const one = oneClaim();
  assert.deepEqual(one.fixAll, today().fixAll);
  assert.equal(one.checkAll, null);
  const none = noClaim();
  assert.equal(none.fixAll, null);
  assert.equal(none.checkAll, null);
});

test("the live results.json gives facts that hold together, or none", () => {
  // A refresh may publish a run too close to call, which leaves a race out;
  // it must never give one that contradicts itself.
  const f = factsFromBenchmarks(live());
  for (const r of [f?.fixAll, f?.checkAll]) {
    if (!r) continue;
    const [hk, ...rivals] = r.rows;
    assert.equal(hk.key, "hk");
    assert.ok(rivals.length > 0);
    assert.equal(r.claim.rival, rivals[0]);
    assert.ok(separated(hk, rivals[0]));
    assert.ok(Number(r.claim.ratio) > 1);
    assert.ok(claimLine(r).length <= 36);
    for (let i = 1; i < rivals.length; i++) assert.ok(rivals[i - 1].median <= rivals[i].median);
    for (const row of r.rows) assert.ok(row.max <= r.axis);
  }
});

/** A scene's caption lines under `facts`, as plain text. */
function lines(id: SectionId, facts: ReelFacts | null): string[] {
  const scene = scenes.find((s) => s.id === id);
  return (scene?.captions?.(facts) ?? []).flatMap((c) => c.lines.map((l) => plain(l.text)));
}

test("no caption shows a number the facts do not back", () => {
  const good = today();
  const every = scenes.map((s) => s.id);
  const others = every.filter((id) => id !== "race");
  // Only the race states numbers, and only its claims' ratios.
  for (const id of others) for (const line of lines(id, good)) assert.doesNotMatch(line, /\d/, `${id}: "${line}"`);
  const withheld: [string, ReelFacts | null, string[]][] = [
    ["no facts", null, []],
    ["no claims", noClaim(), []],
    ["no Fix every file", { ...good, fixAll: null }, ["1.8"]],
    ["no Check every file", oneClaim(), ["1.3"]],
    ["both races", good, ["1.3", "1.8"]],
  ];
  for (const [what, facts, allowed] of withheld) {
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
