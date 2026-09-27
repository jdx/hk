// The benchmark race (storyboard §6.9, scenes/race.ts and race-chart.ts):
// every bar leaves the axis on its race's start beat and stops on the beat
// race-timing.ts gives it (the score rings each ding there), at exactly
// median / axis × 1000 px, never past it; the chart draws only the facts'
// own figures; between the commit and Fix every file the rows re-sort, and
// wherever two rows' labels cross one comes forward and the other ducks
// back; without a claim (F0) it draws no figure at all, only hk's output;
// F0's pane is the lit screen; with more tools than capsules the rows close
// up without touching; and nothing is drawn in the captions' band while a
// caption is up (in Chromium).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { test } from "node:test";
import { BEAT, type ReelFacts, sec, W, H } from "../bible";
import { fmt, type Race } from "../facts";
import { raceRuns } from "../race-timing";
import { checkAll } from "../kit/screens";
import { termLayout } from "../kit/term";
import { captions, F0_DETAIL_AT, F0_PANE, scene } from "../scenes/race";
import { barLen, type ChartModel, chartModel, REORDER, RESET, REST, rowExtent, rowPos, SCALE, slotY } from "../scenes/race-chart";
import { timeCaptions } from "../type";
import { factsFor, noClaim, numbers, previous, scenario } from "./published";
import { REPO, SHOWREEL } from "./repo";

const S = sec("race");

/** `f` with `extra` more tools in each race, each slower than the last. */
function withMoreTools(f: ReelFacts, extra: number): ReelFacts {
  const more = (r: Race | null): Race | null => {
    if (!r) return r;
    const slowest = r.rows[r.rows.length - 1];
    const rows = [...r.rows];
    for (let i = 1; i <= extra; i++) {
      const median = slowest.median * (1 + 0.12 * i);
      rows.push({ key: `tool${i}`, label: `tool-${i}`, mode: "sequential", median, min: median * 0.98, max: median * 1.03, shown: fmt(median) });
    }
    return { ...r, rows, axis: Math.max(...rows.map((x) => x.max)) };
  };
  return { ...f, commit: more(f.commit), fixAll: more(f.fixAll) };
}

/** `f` with Fix every file's rivals in `order`, each taking the figures of the row it lands on. */
function withSecondOrder(f: ReelFacts, order: readonly string[]): ReelFacts {
  const fa = f.fixAll!;
  const byKey = Object.fromEntries(fa.rows.map((r) => [r.key, r]));
  const rows = [fa.rows[0], ...order.map((key, i) => ({ ...fa.rows[i + 1], key, label: byKey[key].label, mode: byKey[key].mode }))];
  return { ...f, fixAll: { ...fa, rows, claim: { ...fa.claim, rival: rows[1] } } };
}

/** `f` with `key` running in `mode` in Fix every file: its mode card flips as the rows re-sort. */
function withSecondMode(f: ReelFacts, key: string, mode: string): ReelFacts {
  const fa = f.fixAll!;
  const rows = fa.rows.map((r) => (r.key === key ? { ...r, mode } : r));
  return { ...f, fixAll: { ...fa, rows, claim: { ...fa.claim, rival: rows.find((r) => r.key === fa.claim.rival.key)! } } };
}

/** A made-up mode for lefthook in Fix every file, which no published run gives it. */
const FLIPPED = "parallel: true";

const VARIANTS: [string, ReelFacts | null][] = [
  ["both", factsFor("both")],
  // More tools than capsules: the rows close up.
  ["six tools", withMoreTools(factsFor("both")!, 2)],
  // A tool whose mode differs between the races: its mode card flips.
  ["a mode that changes", withSecondMode(factsFor("both")!, "lefthook", FLIPPED)],
  ["one", factsFor("one")],
  // The other single claim: the commit alone.
  ["commit only", { ...factsFor("both")!, fixAll: null }],
  // Today's Fix every file alone, which then runs first, from b1.
  ["fix only", { ...factsFor("both")!, commit: null }],
  ["no claim", noClaim()],
  ["no facts", null],
];

test("each bar stops on the beat race-timing gives it, at its median to the pixel, and never runs past it", () => {
  for (const [name, facts] of VARIANTS) {
    const runs = raceRuns(facts);
    const m = chartModel(facts);
    assert.equal(m === null, runs.length === 0, `${name}: a chart exactly when a race is backed`);
    if (!m) continue;
    assert.equal(m.races.length, runs.length, name);
    m.races.forEach((r, i) => {
      const run = runs[i];
      assert.equal(r.start, run.start * BEAT, `${name} ${run.race.key}: starts on the beat raceRuns gives`);
      for (const row of run.race.rows) {
        const e = r.entries[row.key];
        const stop = run.stops[row.key] * BEAT;
        assert.equal(e.stop, stop, `${name} ${run.race.key} ${row.key}: stops on raceRuns' beat`);
        const len = (row.median / run.race.axis) * SCALE;
        assert.ok(Math.abs(e.len - len) < 1e-9, `${name} ${row.key}: at rest it is median / axis × ${SCALE} px`);
        assert.equal(e.shown, row.shown);
        // Sample the race on a 240 fps grid: 0 before the start, growing at
        // one shared speed, exactly its median from the stop until the
        // chart resets or leaves, and never past it.
        const until = m.races.length > 1 && i === 0 ? RESET[0] : REST;
        for (let t = r.start - 0.1; t < until; t += 1 / 240) {
          const got: number | null = i === 0 || t >= r.start ? barLen(m, row.key, t) : null;
          if (got === null) continue;
          if (t <= r.start) assert.equal(got, 0, `${name} ${row.key} at ${t}: not before the start`);
          else if (t < stop) {
            assert.ok(got < e.len, `${name} ${row.key} at ${t}: still short of its median before its stop`);
            assert.ok(Math.abs(got - e.rate * (t - r.start)) < 1e-6, `${name} ${row.key} at ${t}: on the shared clock`);
          } else assert.equal(got, e.len, `${name} ${row.key} at ${t}: exactly its median after its stop`);
        }
      }
      // One clock: every bar in a race grows at the same speed, so the tips run flush.
      const rates = Object.values(r.entries).map((e) => e.rate);
      for (const x of rates) assert.ok(Math.abs(x - rates[0]) < 1e-6 * rates[0], `${name} ${run.race.key}: one speed for every bar`);
      // Rows: hk first, then the others by median.
      assert.deepEqual(r.order, run.race.rows.map((x) => x.key));
      assert.equal(r.order[0], "hk");
    });
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
  const m = chartModel(withMoreTools(factsFor("both")!, 2))!;
  assert.equal(m.races[0].order.length, 6);
  assert.equal(m.geom.detail, false, "six rows leave out the modes and whiskers");
});

test("the second race's bars are home on the axis before they start again", () => {
  const m = chartModel(factsFor("both"));
  assert.ok(m && m.races.length === 2);
  for (const key of m.keys) assert.equal(barLen(m, key, RESET[1]), 0, key);
  assert.ok(RESET[1] <= m.races[1].start);
});

/** Every row's place at `lt`: its centre, its layer, and how far it is in front or ducks back. */
const places = (m: ChartModel, lt: number) => m.keys.map((key) => ({ key, ...rowPos(m, key, lt) }));

/** The re-sort sampled at 240 fps, a little either side. */
function* reorderTimes(): Generator<number> {
  for (let t = REORDER[0] - 0.05; t <= REORDER[1] + 0.05; t += 1 / 240) yield t;
}

/**
 * Rows whose labels cross must never read through each other: they draw on
 * different layers, and the one drawn over the other comes forward (a halo)
 * or the one under it ducks back (dimmed), or both. No row is ever both in
 * front and ducking.
 */
function assertCrossingsLayered(name: string, m: ChartModel): void {
  const pitch = slotY(1, 4) - slotY(0, 4);
  for (const t of reorderTimes()) {
    const rows = places(m, t);
    for (const p of rows) assert.ok(p.front === 0 || p.duck === 0, `${name} at ${t.toFixed(4)}: ${p.key} is in front and ducking at once`);
    for (const p of rows) {
      for (const q of rows) {
        if (p.key >= q.key || Math.abs(p.y - q.y) >= pitch / 2) continue;
        const at = `${name} at ${t.toFixed(4)}: ${p.key} and ${q.key}, ${Math.abs(p.y - q.y).toFixed(1)} px apart`;
        assert.ok(Math.abs(p.depth - q.depth) > 0.5, `${at}, cross on one layer (${p.depth.toFixed(2)}, ${q.depth.toFixed(2)})`);
        const [under, over] = p.depth < q.depth ? [p, q] : [q, p];
        assert.ok(over.front > 0.5 || under.duck > 0.5, `${at}, cross with ${over.key} neither in front nor ${under.key} ducking`);
      }
    }
  }
}

test("between the commit and Fix every file the rows re-sort: lefthook falls two places, ducking under prek and pre-commit", () => {
  const m = chartModel(factsFor("both"))!;
  assert.deepEqual(m.races.map((r) => r.race.key), ["fix-staged", "fix-all"]);
  assert.deepEqual(m.races[0].order, ["hk", "lefthook", "prek", "pre-commit"]);
  assert.deepEqual(m.races[1].order, ["hk", "prek", "pre-commit", "lefthook"]);
  // Before the re-sort every row stands on its commit row, and from its end on its Fix every file row, at rest.
  for (const [i, key] of m.races[0].order.entries()) assert.deepEqual(rowPos(m, key, REORDER[0]), { y: slotY(i, 4), depth: 0, front: 0, jump: 0, duck: 0, alpha: 1, first: i });
  for (const [i, key] of m.races[1].order.entries()) {
    const pos = rowPos(m, key, REORDER[1]);
    assert.ok(Math.abs(pos.y - slotY(i, 4)) < 1e-9 && Math.max(pos.front, pos.jump, pos.duck) < 1e-9, `${key} lands on row ${i}`);
    assert.equal(rowPos(m, key, m.races[1].start).y, slotY(i, 4), `${key} is on row ${i} as Fix every file starts`);
  }
  // Midway, lefthook ducks back and never comes forward; prek and pre-commit
  // come forward over it, stepping rather than jumping, and never duck; hk
  // holds still.
  const mid = (REORDER[0] + REORDER[1]) / 2;
  const at = Object.fromEntries(places(m, mid).map((p) => [p.key, p]));
  assert.ok(at.lefthook.duck > 0.9 && at.lefthook.front === 0 && at.lefthook.jump === 0, "lefthook ducks back as it falls");
  for (const key of ["prek", "pre-commit"]) {
    assert.ok(at[key].front > 0.9 && at[key].duck === 0, `${key} comes forward over lefthook`);
    assert.ok(at[key].jump > 0 && at[key].jump < 0.5, `${key} steps forward, it does not jump`);
  }
  assert.deepEqual([at.hk.front, at.hk.jump, at.hk.duck, at.hk.y], [0, 0, 0, slotY(0, 4)]);
  for (const t of reorderTimes()) {
    const pos = Object.fromEntries(places(m, t).map((p) => [p.key, p]));
    assert.ok(pos.lefthook.front === 0 && pos.prek.duck === 0 && pos["pre-commit"].duck === 0 && pos.hk.front === 0 && pos.hk.duck === 0, `at ${t}`);
  }
  assertCrossingsLayered("commit, then Fix every file", m);
});

test("a tool that climbs two places jumps the rows it overtakes, which duck back", () => {
  // The two races run the other way round: lefthook climbs from last to second.
  const both = factsFor("both")!;
  const m = chartModel({ ...both, commit: both.fixAll, fixAll: both.commit })!;
  assert.deepEqual(m.races[0].order, ["hk", "prek", "pre-commit", "lefthook"]);
  assert.deepEqual(m.races[1].order, ["hk", "lefthook", "prek", "pre-commit"]);
  const mid = (REORDER[0] + REORDER[1]) / 2;
  const at = Object.fromEntries(places(m, mid).map((p) => [p.key, p]));
  assert.ok(at.lefthook.jump > 0.9 && at.lefthook.front > 0.9 && at.lefthook.duck === 0, "lefthook jumps as it climbs");
  for (const key of ["prek", "pre-commit"]) assert.ok(at[key].duck > 0.9 && at[key].front === 0 && at[key].jump === 0, `${key} ducks back under lefthook`);
  assertCrossingsLayered("Fix every file, then the commit", m);
});

test("whatever order Fix every file puts the three rivals in, every crossing is layered", () => {
  const both = factsFor("both")!;
  const rivals = both.commit!.rows.slice(1).map((r) => r.key);
  const orders = (keys: string[]): string[][] => (keys.length < 2 ? [keys] : keys.flatMap((k, i) => orders([...keys.slice(0, i), ...keys.slice(i + 1)]).map((rest) => [k, ...rest])));
  const all = orders(rivals);
  assert.equal(all.length, 6);
  for (const order of all) {
    const m = chartModel(withSecondOrder(both, order))!;
    assert.deepEqual(m.races[1].order, ["hk", ...order]);
    assertCrossingsLayered(`${rivals.join(", ")} to ${order.join(", ")}`, m);
  }
  // A one-place swap: the row that climbs steps forward, the row it passes ducks.
  const [a, b, c] = rivals;
  const swap = chartModel(withSecondOrder(both, [b, a, c]))!;
  const mid = Object.fromEntries(places(swap, (REORDER[0] + REORDER[1]) / 2).map((p) => [p.key, p]));
  assert.ok(mid[b].front > 0.9 && mid[b].jump > 0 && mid[b].jump < 0.5 && mid[a].duck > 0.9, "a swap steps forward over a duck");
  assert.ok(mid[c].front === 0 && mid[c].duck === 0, `${c}, which crosses nobody, stays on its level`);
  // A reversal: the middle row, overtaking one row and passed by the other, stays level between them.
  const rev = chartModel(withSecondOrder(both, [c, b, a]))!;
  const at = Object.fromEntries(places(rev, (REORDER[0] + REORDER[1]) / 2).map((p) => [p.key, p]));
  assert.ok(at[c].jump > 0.9 && at[a].duck > 0.9, "the ends jump and duck");
  assert.deepEqual([at[b].front, at[b].jump, at[b].duck, at[b].depth], [0, 0, 0, 0], "the middle row keeps its level");
});

test("a tool whose mode differs between the races flips its mode card as the rows re-sort", () => {
  const facts = withSecondMode(factsFor("both")!, "lefthook", FLIPPED);
  const drawnAt = (lt: number) => {
    const { ctx, texts } = recorder();
    scene.draw(ctx, lt, { W, H, t: S.start + lt, facts });
    return texts;
  };
  const m = chartModel(facts)!;
  assert.equal(m.races[0].entries.lefthook.mode, "sequential");
  assert.ok(drawnAt(REORDER[0] - 0.01).includes("sequential") && !drawnAt(REORDER[0] - 0.01).includes(FLIPPED), "the commit's mode before the re-sort");
  const after = drawnAt(REORDER[1]);
  assert.ok(after.includes(FLIPPED) && !after.includes("sequential"), "Fix every file's mode after it");
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

test("with a claim the chart draws only the facts' figures: the workload, and each drawn race's summary and medians", () => {
  // The file count is the repository's: the commit race timed only its staged files.
  assert.equal(chartModel(factsFor("both"))!.workload, "6,157-file repo · 10 fixers · 8 CPUs");
  for (const variant of ["both", "one"] as const) {
    const facts = factsFor(variant)!;
    const m = chartModel(facts)!;
    // The commit's summary has a number of its own: "About 60 staged files …".
    const allowed = new Set([m.workload, ...m.races.flatMap((r) => [...(/\d/.test(r.race.summary) ? [r.race.summary] : []), ...r.race.rows.map((x) => x.shown)])]);
    const { texts } = textsDrawn(facts);
    const figures = [...texts].filter((t) => /\d/.test(t));
    for (const t of figures) assert.ok(allowed.has(t), `${variant}: "${t}" is not a figure the facts give`);
    for (const t of allowed) assert.ok(texts.has(t), `${variant}: "${t}" is never drawn`);
    // One claim: the withheld race's medians, the previous run's commit, are nowhere.
    if (variant === "one") {
      for (const [tool, r] of Object.entries(scenario(previous(), "fix-staged").results) as [string, { median: number }][]) {
        if (!allowed.has(fmt(r.median))) assert.ok(!texts.has(fmt(r.median)), `one: ${tool}'s commit ${fmt(r.median)} is drawn`);
      }
    }
  }
});

test("F0's pane is the lit screen while it is up, and nothing is lit with a chart or on a bar line", () => {
  const litAt = (facts: ReelFacts | null, lt: number) => scene.lit!(lt, { facts });
  for (const facts of [factsFor("both"), factsFor("one")]) {
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
