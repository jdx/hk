// The landing page lists what each chapter of the reel shows, for readers
// who cannot watch it (describe.ts, under the player in HomeShowreel.vue).
// Its figures are the video's: every number a caption or the race's chart
// puts on screen is in the chapter's text, and the text states no number the
// video does not show, whichever scenario races, or with none. Every caption
// is quoted in its chapter, but the race's claim, which the text states in
// words.

import assert from "node:assert/strict";
import { test } from "node:test";
import { describeChapters } from "../describe";
import { claimLine, factsFromBenchmarks, fmt, races, type ReelFacts } from "../facts";
import { scenes } from "../scenes";
import { F0_DETAIL } from "../scenes/race";
import { FOOT } from "../scenes/race-chart";
import { SECTIONS, type SectionId } from "../timeline";
import { plain } from "../type";
import { alt, live, noClaim, numbers, only, previous, published, scenario, today } from "./published";

/**
 * The figures a chapter of the video shows under `f`: `told` are the ones
 * the page must state, `shown` the details it may leave out. Captions come
 * from the scenes; what the race draws besides them is race-chart.ts's: the
 * workload line and every bar's median, with the race's summary and every
 * row's mode as details. The F0 terminal's `7/7` is hk's own output, not a
 * benchmark figure, and the page states no number for it.
 */
function onScreen(id: SectionId, f: ReelFacts | null): { told: string[]; shown: string[] } {
  const scene = scenes.find((s) => s.id === id);
  const told = (scene?.captions?.(f) ?? []).flatMap((c) => c.lines).flatMap((l) => numbers(plain(l.text)));
  const shown: string[] = [];
  const r = f?.best;
  if (id === "race" && f && r) {
    told.push(...numbers(f.workload.files), String(f.workload.fixers), String(f.workload.cpus));
    told.push(...r.rows.flatMap((row) => numbers(row.shown)));
    shown.push(...numbers(r.summary), ...r.rows.flatMap((row) => numbers(row.mode)));
  }
  return { told, shown };
}

/** Facts the page and the video meet: today's, the live file's, the previous run's, each other candidate, none, and another run. */
function cases(): [string, ReelFacts | null][] {
  const good = today();
  const fixAll = factsFromBenchmarks(only(published(), "fix-all"))?.best;
  assert.ok(good.best && fixAll);
  const [hk, rival] = fixAll.rows;
  const tak = { ...rival, key: "tak", label: "tak", median: 9.05, shown: fmt(9.05) };
  return [
    ["today's run", good],
    ["the live results.json", factsFromBenchmarks(live())],
    ["the previous run", alt()],
    ["today's Fix every file", factsFromBenchmarks(only(published(), "fix-all"))],
    ["today's Check every file", factsFromBenchmarks(only(published(), "check-all"))],
    ["no facts", null],
    ["no claim", noClaim()],
    [
      "another run",
      {
        workload: { files: "12,480", fixers: 7, cpus: 16 },
        best: {
          ...fixAll,
          summary: "Half of the 12,480 files need 2 fixers.",
          rows: [{ ...hk, median: 4.2, shown: fmt(4.2) }, tak],
          claim: { ratio: "2.2", rival: tak },
        },
      },
    ],
  ];
}

test("the page describes every chapter, in the reel's order", () => {
  const described = describeChapters(today());
  assert.deepEqual(
    described.map(({ id, label }) => ({ id, label })),
    SECTIONS.map(({ id, label }) => ({ id, label })),
  );
  for (const { id, text } of described) assert.ok(text.length > 50, `${id}: "${text}"`);
});

test("the page states the figures the video shows, and no others", () => {
  for (const [name, f] of cases()) {
    for (const { id, text } of describeChapters(f)) {
      const page = numbers(text);
      const { told, shown } = onScreen(id, f);
      for (const n of told) assert.ok(page.includes(n), `${name}, ${id}: the video shows ${n}, the page leaves it out: "${text}"`);
      for (const n of page) {
        assert.ok(told.includes(n) || shown.includes(n), `${name}, ${id}: the page says ${n}, the video does not: "${text}"`);
      }
    }
  }
});

test("no chapter but the race has a digit, whatever the facts", () => {
  for (const [name, f] of cases()) {
    for (const { id, text } of describeChapters(f)) if (id !== "race") assert.doesNotMatch(text, /\d/, `${name}, ${id}: "${text}"`);
  }
});

test("today's race reads the frozen run's figures: the commit, in plain words", () => {
  const text = describeChapters(today()).find((c) => c.id === "race")?.text ?? "";
  assert.equal(
    text,
    "A bar chart race from hk's published benchmark, run on a 6,157-file repository with 10 fixers and 8 CPUs. " +
      "Commit: About 60 staged files with defects, fixed by each tool's pre-commit hook. " +
      "hk (parallel, file locks) takes 907 ms, lefthook (sequential) 1.42 s, prek (sequential hooks, batched files) 1.77 s and pre-commit (sequential hooks, batched files) 2.44 s. " +
      "hk is 1.6 times faster than the fastest other tool, lefthook. " +
      "Caption: Timed only when the files are right. " +
      "A note under the chart reads Every tool and scenario: hk.jdx.dev/benchmarks.",
  );
});

test("the previous run's race reads Check every file's figures, with lefthook's mode in that scenario", () => {
  const text = describeChapters(alt()).find((c) => c.id === "race")?.text ?? "";
  assert.equal(
    text,
    "A bar chart race from hk's published benchmark, run on a 6,157-file repository with 10 fixers and 8 CPUs. " +
      "Check every file: Every file is already clean and nothing writes, so parallel is safe for every tool. " +
      "hk (parallel, file locks) takes 3.82 s, lefthook (parallel: true) 6.77 s, prek (sequential hooks, batched files) 7.55 s and pre-commit (sequential hooks, batched files) 8.19 s. " +
      "hk is 1.8 times faster than the fastest other tool, lefthook. " +
      "Caption: Timed only when the files are right. " +
      "A note under the chart reads Every tool and scenario: hk.jdx.dev/benchmarks.",
  );
});

test("the race quotes its caption and the bottom note; no race describes hk's run and quotes the pointer, with no number", () => {
  const race = (f: ReelFacts | null) => describeChapters(f).find((c) => c.id === "race")?.text ?? "";
  // The chart's bottom detail is quoted as the chart draws it.
  for (const f of [today(), alt()]) assert.ok(race(f).endsWith(`reads ${FOOT}.`), race(f));
  // No facts and a run that backs no claim draw the same terminal, and the
  // page says nothing about the run that is true of only one of them.
  for (const f of [null, noClaim()]) {
    assert.equal(
      race(f),
      "A terminal runs hk check --all: six of its seven steps start together, ruff-format follows ruff, and all seven pass. " +
        "Caption: Independent steps run in parallel. A note under the terminal reads Benchmarks: hk.jdx.dev/benchmarks.",
    );
    // The note is quoted as the scene draws it.
    assert.ok(race(f).includes(`reads ${F0_DETAIL}.`), F0_DETAIL);
  }
});

test("every caption is quoted in its chapter, and a claim's ratio stated in words", () => {
  for (const [name, f] of cases()) {
    const described = describeChapters(f);
    for (const s of scenes) {
      const text = described.find((c) => c.id === s.id)?.text ?? "";
      for (const c of s.captions?.(f) ?? []) {
        const quote = `Caption: ${c.lines.map((l) => plain(l.text)).join(" ")}`;
        const claim = s.id === "race" ? races(f).find((r) => quote.includes(claimLine(r))) : undefined;
        if (claim) assert.ok(text.includes(`${claim.claim.ratio} times faster than the fastest other tool`), `${name}, ${s.id}: "${text}"`);
        else assert.ok(text.includes(quote), `${name}, ${s.id}: "${quote}" is not quoted in "${text}"`);
      }
    }
  }
});

test("a scenario the video does not race is never on the page", () => {
  const pageOf = (f: ReelFacts) =>
    describeChapters(f)
      .map((c) => c.text)
      .join(" ");
  const absent = (page: string, run: ReturnType<typeof published>, key: string) => {
    const s = scenario(run, key);
    assert.ok(!page.includes(s.summary), `${key}'s summary is on the page`);
    for (const r of Object.values(s.results) as { median: number }[]) assert.ok(!page.includes(fmt(r.median)), `${key}'s ${fmt(r.median)} is on the page`);
  };
  absent(pageOf(today()), published(), "fix-all");
  absent(pageOf(today()), published(), "check-all");
  // The previous run's commit, which lefthook won, and its Fix every file.
  absent(pageOf(alt()), previous(), "fix-staged");
  absent(pageOf(alt()), previous(), "fix-all");
});
