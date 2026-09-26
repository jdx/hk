// The benchmark race (storyboard §6.9, scenes/race.ts and race-chart.ts):
// every bar leaves the axis on its race's start beat and stops on the beat
// race-timing.ts gives it (the score rings each ding there), at exactly
// median / axis × 1000 px, never past it; the chart draws only the facts'
// own figures; without a claim (F0) it draws no figure at all, only hk's
// output; F0's pane is the lit screen; and nothing is drawn in the
// captions' band while a caption is up (in Chromium).

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { test } from "node:test";
import { BEAT, type ReelFacts, sec, W, H } from "../bible";
import { raceRuns } from "../race-timing";
import { checkAll } from "../kit/screens";
import { PANE_FULL } from "../kit/term";
import { captions, scene } from "../scenes/race";
import { barLen, chartModel, RESET, REST, SCALE } from "../scenes/race-chart";
import { timeCaptions } from "../type";
import { factsFor, noClaim, numbers } from "./published";
import { REPO, SHOWREEL } from "./repo";

const S = sec("race");
const VARIANTS: [string, ReelFacts | null][] = [
  ["both", factsFor("both")],
  ["one", factsFor("one")],
  // The other single claim: Check every file alone, which then runs first, from b1.
  ["check only", { ...factsFor("both")!, fixAll: null }],
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

test("the second race's bars are home on the axis before they start again", () => {
  const m = chartModel(factsFor("both"));
  assert.ok(m && m.races.length === 2);
  for (const key of m.keys) assert.equal(barLen(m, key, RESET[1]), 0, key);
  assert.ok(RESET[1] <= m.races[1].start);
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

test("with a claim the chart draws only the facts' figures: the workload and each drawn race's medians", () => {
  for (const variant of ["both", "one"] as const) {
    const facts = factsFor(variant)!;
    const m = chartModel(facts)!;
    const allowed = new Set([m.workload, ...m.races.flatMap((r) => r.race.rows.map((x) => x.shown))]);
    const { texts } = textsDrawn(facts);
    const figures = [...texts].filter((t) => /\d/.test(t));
    for (const t of figures) assert.ok(allowed.has(t), `${variant}: "${t}" is not a figure the facts give`);
    for (const t of allowed) assert.ok(texts.has(t), `${variant}: "${t}" is never drawn`);
    // One claim: the other race's medians are nowhere.
    if (variant === "one") {
      const other = factsFor("both")!.checkAll!;
      for (const x of other.rows) if (!allowed.has(x.shown)) assert.ok(!texts.has(x.shown), `one: ${x.label}'s ${x.shown} is drawn`);
    }
  }
});

test("F0's pane is the lit screen while it is up, and nothing is lit with a chart or on a bar line", () => {
  const { ctx } = recorder();
  const litAt = (facts: ReelFacts | null, lt: number) => {
    scene.draw(ctx, lt, { W, H, t: S.start + lt, facts });
    return scene.lit!(lt);
  };
  for (const facts of [factsFor("both"), factsFor("one")]) {
    for (let lt = 0; lt < S.len; lt += BEAT / 4) assert.equal(litAt(facts, lt), null, `lit at ${lt} with a chart`);
  }
  assert.equal(litAt(null, 0), null, "nothing lit on the whip's bar line");
  assert.equal(litAt(null, S.len - 1 / 120), null, "nothing lit on race|morph");
  const up = litAt(null, 4 * BEAT);
  assert.deepEqual(up, { x: PANE_FULL.x, y: PANE_FULL.y, w: PANE_FULL.w, h: PANE_FULL.h, alpha: 1 });
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
