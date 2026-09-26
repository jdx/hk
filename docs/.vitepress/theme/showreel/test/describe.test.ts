// The landing page lists what each chapter of the reel shows, for readers
// who cannot watch it (describe.ts, under the player in HomeShowreel.vue).
// Its figures are the video's: every number a caption or the race's chart
// puts on screen is in the chapter's text, and the text states no number the
// video does not show, with both races, one, or none. Every caption is
// quoted in its chapter, but a race's claim, which the text states in words.

import assert from "node:assert/strict";
import { test } from "node:test";
import { describeChapters } from "../describe";
import { claimLine, factsFromBenchmarks, fmt, races, type ReelFacts } from "../facts";
import { scenes } from "../scenes";
import { F0_DETAIL } from "../scenes/race";
import { SECTIONS, type SectionId } from "../timeline";
import { plain } from "../type";
import { live, noClaim, numbers, oneClaim, published, scenario, today } from "./published";

/**
 * The figures a chapter of the video shows under `f`: `told` are the ones
 * the page must state, `shown` the details it may leave out. Captions come
 * from the scenes; what the race draws besides them is race-chart.ts's: the
 * workload line and every bar's median, with each race's summary and every
 * row's mode as details. The F0 terminal's `7/7` is hk's own output, not a
 * benchmark figure, and the page states no number for it.
 */
function onScreen(id: SectionId, f: ReelFacts | null): { told: string[]; shown: string[] } {
  const scene = scenes.find((s) => s.id === id);
  const told = (scene?.captions?.(f) ?? []).flatMap((c) => c.lines).flatMap((l) => numbers(plain(l.text)));
  const shown: string[] = [];
  const drawn = races(f);
  if (id === "race" && f && drawn.length) {
    told.push(...numbers(f.workload.files), String(f.workload.fixers), String(f.workload.cpus));
    for (const r of drawn) {
      told.push(...r.rows.flatMap((row) => numbers(row.shown)));
      shown.push(...numbers(r.summary), ...r.rows.flatMap((row) => numbers(row.mode)));
    }
  }
  return { told, shown };
}

/** Facts the page and the video meet: today's, the live file's, one race, none, and another run. */
function cases(): [string, ReelFacts | null][] {
  const good = today();
  assert.ok(good.fixAll && good.checkAll);
  const hk = good.fixAll.rows[0];
  return [
    ["today's run", good],
    ["the live results.json", factsFromBenchmarks(live())],
    ["no facts", null],
    ["no claims", noClaim()],
    ["only Fix every file", oneClaim()],
    ["only Check every file", { ...good, fixAll: null }],
    [
      "another run",
      {
        workload: { files: "12,480", fixers: 7, cpus: 16 },
        fixAll: {
          ...good.fixAll,
          summary: "Half of the 12,480 files need 2 fixers.",
          rows: [
            { ...hk, median: 4.2, shown: fmt(4.2) },
            { ...good.fixAll.rows[1], key: "tak", label: "tak", median: 9.05, shown: fmt(9.05) },
          ],
          claim: { ratio: "2.2", rival: { ...good.fixAll.rows[1], key: "tak", label: "tak", median: 9.05, shown: fmt(9.05) } },
        },
        checkAll: null,
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

test("today's race reads the frozen run's figures", () => {
  const text = describeChapters(today()).find((c) => c.id === "race")?.text ?? "";
  assert.equal(
    text,
    "A bar chart race from hk's published benchmark: 6,157 files, 10 fixers, 8 CPUs. " +
      "To fix every file, hk (parallel, file locks) takes 10.6 s, prek (sequential hooks, batched files) 13.7 s, pre-commit (sequential hooks, batched files) 14.6 s and lefthook (sequential) 24.9 s: 1.3 times faster than the fastest other tool, prek. " +
      "To check every file, hk (parallel, file locks) takes 3.82 s, lefthook (parallel: true) 6.77 s, prek (sequential hooks, batched files) 7.55 s and pre-commit (sequential hooks, batched files) 8.19 s: 1.8 times faster than the fastest other tool, lefthook. " +
      "Commit timings and every tool are on hk.jdx.dev/benchmarks.",
  );
});

test("one race says why the other is missing; none describes hk's run and quotes the pointer, with no number", () => {
  const race = (f: ReelFacts | null) => describeChapters(f).find((c) => c.id === "race")?.text ?? "";
  const one = race(oneClaim());
  assert.match(one, /To fix every file, hk \(parallel, file locks\) takes 10\.6 s/);
  assert.doesNotMatch(one, /check every file/);
  assert.match(one, /Caption: Timed only when the files are right\. Commit timings and every tool are on hk\.jdx\.dev\/benchmarks\.$/);
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

test("the commit scenario's timings are never on the page", () => {
  const staged = scenario(published(), "fix-staged");
  const page = describeChapters(today())
    .map((c) => c.text)
    .join(" ");
  for (const r of Object.values(staged.results) as { median: number }[]) assert.ok(!page.includes(fmt(r.median)), fmt(r.median));
});
