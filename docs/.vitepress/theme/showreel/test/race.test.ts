// The benchmark race (storyboard §6.9, scenes/race.ts and race-chart.ts):
// every bar leaves the axis on the race's start beat and stops on the beat
// race-timing.ts gives it (the score rings each ding there), at exactly
// median / axis × 1000 px, never past it; the claim lands only once the bars
// it compares have stopped; the chart draws only the facts' own figures;
// without a claim (F0) it draws no figure at all, only hk's output; F0's
// pane is the lit screen; with more tools than capsules the rows close up
// without touching; and nothing is drawn in the captions' band while a
// caption is up (in Chromium).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { test } from "node:test";
import { BEAT, type ReelFacts, sec, W, H } from "../bible";
import { claimLine, factsFromBenchmarks, fmt, type RaceKey } from "../facts";
import { raceRun } from "../race-timing";
import { checkAll } from "../kit/screens";
import { termLayout } from "../kit/term";
import { captions, F0_DETAIL_AT, F0_PANE, scene } from "../scenes/race";
import { barLen, chartModel, type Entry, REST, ROLL_CALL, rollCallAt, rowExtent, SCALE, slotY } from "../scenes/race-chart";
import { timeCaptions, WORD } from "../type";
import { alt, factsFor, noClaim, numbers, only, previous, published, scenario, today } from "./published";
import { REPO, SHOWREEL } from "./repo";

const S = sec("race");

/** `f` with `extra` more tools in its race, each slower than the last. */
function withMoreTools(f: ReelFacts, extra: number): ReelFacts {
  const r = f.best;
  if (!r) return f;
  const slowest = r.rows[r.rows.length - 1];
  const rows = [...r.rows];
  for (let i = 1; i <= extra; i++) {
    const median = slowest.median * (1 + 0.12 * i);
    rows.push({ key: `tool${i}`, label: `tool-${i}`, mode: "sequential", median, min: median * 0.98, max: median * 1.03, shown: fmt(median) });
  }
  return { ...f, best: { ...r, rows, axis: Math.max(...rows.map((x) => x.max)) } };
}

const VARIANTS: [string, ReelFacts | null][] = [
  ["today", today()],
  ["alt", alt()],
  // Every other candidate either run has, raced alone: other claims, other rivals, other stops.
  ["today's Fix every file", factsFromBenchmarks(only(published(), "fix-all"))],
  ["today's Check every file", factsFromBenchmarks(only(published(), "check-all"))],
  ["the previous run's Fix every file", factsFromBenchmarks(only(previous(), "fix-all"))],
  // More tools than capsules: the rows close up.
  ["six tools", withMoreTools(today(), 2)],
  ["no claim", noClaim()],
  ["no facts", null],
];

test("each bar stops on the beat race-timing gives it, at its median to the pixel, and never runs past it", () => {
  for (const [name, facts] of VARIANTS) {
    const run = raceRun(facts);
    const m = chartModel(facts);
    assert.equal(m === null, run === null, `${name}: a chart exactly when a race is backed`);
    if (!m || !run) continue;
    assert.equal(m.start, run.start * BEAT, `${name}: starts on the beat raceRun gives`);
    for (const row of run.race.rows) {
      const e: Entry = m.entries[row.key];
      const stop: number = run.stops[row.key] * BEAT;
      assert.equal(e.stop, stop, `${name} ${row.key}: stops on raceRun's beat`);
      const len: number = (row.median / run.race.axis) * SCALE;
      assert.ok(Math.abs(e.len - len) < 1e-9, `${name} ${row.key}: at rest it is median / axis × ${SCALE} px`);
      assert.equal(e.shown, row.shown);
      // Sample the race on a 240 fps grid: 0 before the start, growing at
      // one shared speed, exactly its median from the stop until the chart
      // leaves, and never past it.
      for (let t: number = m.start - 0.1; t < REST; t += 1 / 240) {
        const got: number = barLen(m, row.key, t);
        if (t <= m.start) assert.equal(got, 0, `${name} ${row.key} at ${t}: not before the start`);
        else if (t < stop) {
          assert.ok(got < e.len, `${name} ${row.key} at ${t}: still short of its median before its stop`);
          assert.ok(Math.abs(got - e.rate * (t - m.start)) < 1e-6, `${name} ${row.key} at ${t}: on the shared clock`);
        } else assert.equal(got, e.len, `${name} ${row.key} at ${t}: exactly its median after its stop`);
      }
    }
    // One clock: every bar grows at the same speed, so the tips run flush.
    const rates = Object.values(m.entries).map((e) => e.rate);
    for (const x of rates) assert.ok(Math.abs(x - rates[0]) < 1e-6 * rates[0], `${name}: one speed for every bar`);
    // Rows: hk first, then the others by median; the slowest stops 2.5 beats after the start.
    assert.deepEqual(m.order, run.race.rows.map((x) => x.key));
    assert.equal(m.order[0], "hk");
    assert.equal(Math.max(...Object.values(run.stops)), run.start + 2.5);
  }
});

test("however many tools race, no row's ink reaches the next row's", () => {
  for (let n = 2; n <= 10; n++) {
    const { above, below } = rowExtent(n);
    for (let i = 0; i + 1 < n; i++) {
      const pitch = slotY(i + 1, n) - slotY(i, n);
      assert.ok(below + above < pitch, `${n} rows: rows ${i} and ${i + 1} are ${pitch} px apart, but a row reaches ${above} px up and ${below} px down`);
    }
  }
  // The six-tool fixture draws every row, each in its own band.
  const m = chartModel(withMoreTools(today(), 2))!;
  assert.equal(m.order.length, 6);
  assert.equal(m.geom.detail, false, "six rows leave out the modes and whiskers");
});

test("the claim's ratio rises only once both bars it compares have stopped, and the note once the claim is leaving", () => {
  for (const [name, facts] of VARIANTS) {
    const run = raceRun(facts);
    if (!run) continue;
    const [claim, note] = timeCaptions(S, captions(facts));
    const line = claim.lines[0];
    assert.equal(line.text, claimLine(run.race));
    // The ratio is the line's second-to-last word, so it starts to rise two words before the line lands.
    assert.equal(line.text.split(" ").at(-2), `${run.race.claim.ratio}×`);
    const rises = line.land - 2 * WORD;
    for (const key of ["hk", run.race.claim.rival.key]) {
      const stop = S.beat(run.stops[key]);
      assert.ok(rises >= stop + BEAT / 4 - 1e-9, `${name}: the ${run.race.claim.ratio}× rises at ${rises.toFixed(3)} s, before ${key}'s bar has stopped at ${stop.toFixed(3)} s`);
    }
    assert.ok(note.start >= claim.out - 1e-9, `${name}: the note rises before the claim leaves`);
    // The roll call runs down the bars as the note lands, the first a sixteenth before it.
    assert.equal(S.start + rollCallAt(run, 0), note.lines[0].land - ROLL_CALL.ahead);
  }
  // Today's commit keeps the one-race layout's beats; the previous run's longer claim, whose rival stops later, holds longer.
  const beats = (f: ReelFacts) => {
    const run = raceRun(f)!;
    return { claim: run.claim, note: run.note };
  };
  assert.deepEqual(beats(today()), { claim: { in: 3, out: 8.5 }, note: 9.75 });
  assert.deepEqual(beats(alt()), { claim: { in: 4, out: 10 }, note: 11 });
});

// A 2D context that draws nothing and records every string filled, for the
// figures a frame shows.
function recorder(): { ctx: CanvasRenderingContext2D; texts: string[] } {
  const texts: string[] = [];
  const store: Record<string | symbol, unknown> = {};
  const noop = () => undefined;
  const gradient = { addColorStop: noop };
  let ctx: CanvasRenderingContext2D;
  const handler: ProxyHandler<object> = {
    get(_, key) {
      if (key in store) return store[key];
      switch (key) {
        case "fillText":
          return (text: string) => void texts.push(String(text));
        case "measureText":
          return (text: string) => ({ width: String(text).length * 20 });
        case "getTransform":
          return () => ({ a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 });
        case "createLinearGradient":
        case "createRadialGradient":
          return () => gradient;
        case "createPattern":
          return () => null;
        case "canvas":
          return { width: W, height: H, getContext: () => ctx };
        default:
          return noop;
      }
    },
    set(_, key, value) {
      store[key] = value;
      return true;
    },
  };
  ctx = new Proxy({}, handler) as CanvasRenderingContext2D;
  // fx.ts makes its sprites on canvases of its own.
  (globalThis as { document?: unknown }).document ??= { createElement: () => ({ width: 0, height: 0, getContext: () => ctx }) };
  return { ctx, texts };
}

/** Every string the scene fills over the section, a 32nd apart, under `facts`. */
function textsDrawn(facts: ReelFacts | null): { ctx: CanvasRenderingContext2D; texts: Set<string> } {
  const { ctx, texts } = recorder();
  for (let lt = 0; lt < S.len; lt += BEAT / 8) scene.draw(ctx, lt, { W, H, t: S.start + lt, facts });
  return { ctx, texts: new Set(texts) };
}

test("without a claim (F0) the scene draws no benchmark figure: every digit is hk's own output", () => {
  const output = checkAll.flat();
  for (const facts of [null, noClaim()]) {
    const { texts } = textsDrawn(facts);
    assert.ok(texts.size > 0, "the terminal draws");
    for (const text of texts) {
      if (!/\d/.test(text)) continue;
      assert.ok(
        output.some((line) => line.includes(text)),
        `"${text}" (${facts ? "no claim" : "no facts"}) is not hk's check --all output`,
      );
    }
    for (const c of captions(facts)) for (const l of c.lines) assert.deepEqual(numbers(l.text), [], l.text);
  }
});

test("with a claim the chart draws only the facts' figures: the workload, the race's summary and its medians", () => {
  // The file count is the repository's: the commit race timed only its staged files.
  assert.equal(chartModel(today())!.workload, "6,157-file repo · 10 fixers · 8 CPUs");
  // The scenarios each run does not race: none of their medians is drawn.
  const unraced: [string, ReelFacts, ReturnType<typeof published>, RaceKey[]][] = [
    ["today", today(), published(), ["fix-all", "check-all"]],
    ["alt", alt(), previous(), ["fix-staged", "fix-all"]],
  ];
  for (const [variant, facts, run, others] of unraced) {
    const m = chartModel(facts)!;
    // The commit's summary has a number of its own: "About 60 staged files …".
    const allowed = new Set([m.workload, ...(/\d/.test(m.race.summary) ? [m.race.summary] : []), ...m.race.rows.map((x) => x.shown)]);
    const { texts } = textsDrawn(facts);
    const figures = [...texts].filter((t) => /\d/.test(t));
    for (const t of figures) assert.ok(allowed.has(t), `${variant}: "${t}" is not a figure the facts give`);
    for (const t of allowed) assert.ok(texts.has(t), `${variant}: "${t}" is never drawn`);
    for (const key of others) {
      for (const [tool, r] of Object.entries(scenario(run, key).results) as [string, { median: number }][]) {
        if (!allowed.has(fmt(r.median))) assert.ok(!texts.has(fmt(r.median)), `${variant}: ${tool}'s ${key} ${fmt(r.median)} is drawn`);
      }
    }
  }
  // Each row's mode is the scenario's: lefthook ran Check every file with "parallel: true".
  assert.ok(textsDrawn(alt()).texts.has("parallel: true"));
});

test("F0's pane is the lit screen while it is up, and nothing is lit with a chart or on a bar line", () => {
  const litAt = (facts: ReelFacts | null, lt: number) => scene.lit!(lt, { facts });
  for (const facts of [factsFor("today"), factsFor("alt")]) {
    for (let lt = 0; lt < S.len; lt += BEAT / 4) assert.equal(litAt(facts, lt), null, `lit at ${lt} with a chart`);
  }
  assert.equal(litAt(null, 0), null, "nothing lit on the whip's bar line");
  assert.equal(litAt(null, S.len - 1 / 120), null, "nothing lit on race|morph");
  const up = litAt(null, 4 * BEAT);
  assert.deepEqual(up, { x: F0_PANE.x, y: F0_PANE.y, w: F0_PANE.w, h: F0_PANE.h, alpha: 1 });
});

test("F0's pane shows every row of every checkAll screen, inside its window, with the pointer to the page under it", () => {
  const most = Math.max(...checkAll.map((s) => s.length));
  assert.ok(F0_PANE.rows >= most, `the pane holds ${F0_PANE.rows} rows, but a screen has ${most}`);
  const L = termLayout(F0_PANE, most);
  assert.equal(L.first, 0, "top rows first");
  // Ascenders reach 0.8 em above a baseline and descenders 0.25 em below.
  assert.ok(L.baseline(0) - 0.8 * F0_PANE.size >= L.body.y, "the first row is inside the window's body");
  const bottom = F0_PANE.y + F0_PANE.h;
  assert.ok(L.baseline(most - 1) + 0.25 * F0_PANE.size <= bottom, "the last row is inside the window");
  assert.ok(F0_PANE.y >= 100 && bottom <= 700, "the pane stands in the actors' zone");
  // The 40 px pointer: its caps clear the pane, its descenders the captions' band.
  assert.ok(F0_DETAIL_AT.y - 0.75 * 40 >= bottom + 12, "the pointer clears the pane");
  assert.ok(F0_DETAIL_AT.y + 0.25 * 40 <= 740 - 8, "the pointer clears the captions' band");
});

// In Chromium: the captions' band stays empty while a caption is up.

const load = createRequire(join(REPO, "docs/package.json"));

test("in Chromium, nothing is drawn in the captions' band while a race caption is up", async (t) => {
  const { chromium } = load("playwright-core") as typeof import("playwright-core");
  const { build } = load("esbuild") as typeof import("esbuild");
  let browser: import("playwright-core").Browser;
  try {
    browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined });
  } catch (err) {
    if (process.env.SHOWREEL_REQUIRE_CHROMIUM) throw err;
    t.skip(`no Chromium to draw in: ${String(err).split("\n")[0]}`);
    return;
  }
  try {
    const bundle = await build({
      stdin: {
        contents: `export { composeReel } from "./compose";
export { scene } from "./scenes/race";
export { resetTypeCache } from "./type";`,
        resolveDir: SHOWREEL,
        loader: "ts",
      },
      bundle: true,
      format: "iife",
      globalName: "Race",
      target: "es2022",
      write: false,
      logLevel: "error",
    });
    const fonts = Object.fromEntries(
      ["SpaceGrotesk.ttf", "LiberationMono-Regular.ttf", "LiberationMono-Bold.ttf"].map((f) => [
        f,
        readFileSync(join(REPO, "docs/.vitepress/fonts", f)).toString("base64"),
      ]),
    );
    const page = await browser.newPage();
    const errors: Error[] = [];
    page.on("pageerror", (err) => errors.push(err));
    await page.setContent("<body></body>");
    await page.addScriptTag({ content: bundle.outputFiles[0].text });
    await page.evaluate(async (fonts) => {
      const bytes = (b64: string) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
      const faces = [
        new FontFace("Space Grotesk", bytes(fonts["SpaceGrotesk.ttf"]), { weight: "300 700" }),
        new FontFace("Liberation Mono", bytes(fonts["LiberationMono-Regular.ttf"]), { weight: "400" }),
        new FontFace("Liberation Mono", bytes(fonts["LiberationMono-Bold.ttf"]), { weight: "700" }),
      ];
      for (const face of faces) document.fonts.add(await face.load());
      Race.resetTypeCache();
    }, fonts);
    for (const [name, facts] of VARIANTS) {
      // Every 64th while any line of a caption is on screen, from its first word to the end of its wipe.
      const times: number[] = [];
      for (const c of timeCaptions(S, captions(facts))) for (let at = c.start; at < c.end; at += BEAT / 16) times.push(at);
      const bad = await page.evaluate(
        ({ facts, times }) => {
          const reel = Race.composeReel([Race.scene], facts, { raw: true });
          const ctx = document.createElement("canvas").getContext("2d", { alpha: false })!;
          ctx.canvas.width = 1920;
          ctx.canvas.height = 1080;
          const out: string[] = [];
          for (const at of times) {
            reel.render(ctx, at, 1920, 1080);
            const d = ctx.getImageData(0, 740, 1920, 226).data;
            // The stage's own colour, bg #0c151d, within a level.
            let over = 0;
            for (let i = 0; i < d.length; i += 4) if (Math.abs(d[i] - 12) > 1 || Math.abs(d[i + 1] - 21) > 1 || Math.abs(d[i + 2] - 29) > 1) over++;
            if (over) out.push(`${at.toFixed(3)} s: ${over} px`);
          }
          return out;
        },
        { facts, times },
      );
      assert.deepEqual(bad, [], `${name}: scene pixels in the captions' band while a caption is up`);
    }
    assert.deepEqual(errors, [], "no frame throws");
  } finally {
    await browser.close();
  }
});

declare const Race: typeof import("../compose") & { scene: typeof scene } & typeof import("../type");
