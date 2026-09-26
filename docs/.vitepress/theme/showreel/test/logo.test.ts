// K1, the marks (storyboard §4 K1, identity §1). The pure half checks the
// geometry: the strokes are logo.svg's, the hard-coded lengths are the
// paths' lengths, the placements put the ink where the storyboard says, and
// the swing settles when it says. The Chromium half draws: drawLogo at full
// progress against docs/public/logo.svg drawn by the same browser at the
// same size (they may differ only where antialiasing does), a docked hook
// hidden under the wordmark, the free hook against the favicon, and fades
// that do not double up. Skipped when no Chromium is installed
// (`aube exec playwright-core install chromium-headless-shell`, or a
// Chromium at CHROME_PATH), unless SHOWREEL_REQUIRE_CHROMIUM is set: CI
// sets it whenever it installs one, so a broken install fails here instead
// of skipping the check.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { test } from "node:test";
import { BAR, BEAT, sec } from "../bible";
import {
  hookAnchors,
  hookPlace,
  KLEG_RUN,
  LOGO_END,
  LOGO_INK,
  LOGO_OPEN,
  LOGO_POINT,
  LOGO_STROKES,
  logoScale,
  logoToPx,
  pathLength,
  penStrokes,
  settledSwing,
  STROKE,
  strokeTip,
  swingDeg,
} from "../kit/logo";
import { REPO, SHOWREEL } from "./repo";

const SVG = readFileSync(join(REPO, "docs/public/logo.svg"), "utf8");
const near = (got: number, want: number, tol: number, what: string) =>
  assert.ok(Math.abs(got - want) <= tol, `${what}: ${got} is not within ${tol} of ${want}`);
const b = (n: number) => n * BEAT;

test("the strokes are logo.svg's paths, the arm reversed and the barb split from the leg", () => {
  const paths = [...SVG.matchAll(/<path d="([^"]+)"/g)].map((m) => m[1]);
  assert.equal(paths.length, 5);
  const d = LOGO_STROKES.map((s) => s.d);
  assert.equal(d[STROKE.hStem], paths[0]);
  assert.equal(d[STROKE.hShoulder], paths[1]);
  assert.equal(d[STROKE.kStem], paths[2]);
  assert.equal(d[STROKE.kArm], "M92 68 L126 46");
  assert.equal(paths[3], "M126 46 L92 68");
  assert.equal(`${d[STROKE.kLeg]} ${d[STROKE.kBarb]}`, paths[4]);
  assert.match(SVG, /stroke="#4ADEF0" stroke-width="10" stroke-linecap="round" stroke-linejoin="round"/);
  assert.match(SVG, /viewBox="10 4 140 112"/);
});

test("the hard-coded lengths are the paths' lengths", () => {
  for (const s of LOGO_STROKES) near(pathLength(s.d), s.len, 0.01, s.id);
  // identity §1.2: the leg is a straight run, a cubic, a half circle and 6 up.
  near(KLEG_RUN, 26.08, 0.005, "the leg's straight run");
  near(pathLength("M122 76 C130 81 132 87 132 92"), 19.77, 0.005, "the leg's cubic");
  near(pathLength("M132 92 A13 13 0 0 1 106 92"), 13 * Math.PI, 1e-9, "the bowl");
  near(pathLength("M28 58 A16 16 0 0 1 60 58"), 16 * Math.PI, 1e-9, "the shoulder's arch");
});

test("the pen's tip runs from each stroke's start to its end", () => {
  const ends: Record<string, [[number, number], [number, number]]> = {
    hStem: [[28, 16], [28, 80]],
    hShoulder: [[28, 58], [60, 80]],
    kStem: [[92, 16], [92, 80]],
    kArm: [[92, 68], [126, 46]],
    kLeg: [[100, 62], [106, 86]],
    kBarb: [[106, 86], [112, 91]],
  };
  LOGO_STROKES.forEach((s, i) => {
    const [a, z] = ends[s.id];
    const p0 = strokeTip(i, 0);
    const p1 = strokeTip(i, 1);
    near(p0.x, a[0], 1e-9, `${s.id} start x`);
    near(p0.y, a[1], 1e-9, `${s.id} start y`);
    near(p1.x, z[0], 1e-6, `${s.id} end x`);
    near(p1.y, z[1], 1e-6, `${s.id} end y`);
  });
  // The arch's top, half way round the shoulder's half circle.
  const top = strokeTip(STROKE.hShoulder, (8 * Math.PI) / 72.27);
  near(top.x, 44, 0.02, "the arch's top x");
  near(top.y, 42, 0.02, "the arch's top y");
  // Where morph swaps its last capsule for the leg: the end of the straight run.
  const run = strokeTip(STROKE.kLeg, KLEG_RUN / LOGO_STROKES[STROKE.kLeg].len);
  near(run.x, 122, 0.02, "the run's end x");
  near(run.y, 76, 0.02, "the run's end y");
  // The bowl's bottom.
  const bottom = strokeTip(STROKE.kLeg, (KLEG_RUN + 19.77 + (13 * Math.PI) / 2) / 92.69);
  near(bottom.x, 119, 0.05, "the bowl's bottom x");
  near(bottom.y, 105, 0.05, "the bowl's bottom y");
});

test("the placements put the ink where the storyboard says", () => {
  const box = (place: typeof LOGO_OPEN) => {
    const a = logoToPx(place, LOGO_INK.x0, LOGO_INK.y0);
    const z = logoToPx(place, LOGO_INK.x1, LOGO_INK.y1);
    return [a.x, z.x, a.y, z.y];
  };
  const open = box(LOGO_OPEN);
  [817.5, 1102.5, 277.5, 525].forEach((v, i) => near(open[i], v, 1e-9, `LOGO_OPEN ink ${i}`));
  near(logoScale(LOGO_OPEN) * 10, 25, 1e-9, "LOGO_OPEN stroke");
  const end = box(LOGO_END);
  [1196, 1644, 137.5, 526.4].forEach((v, i) => near(end[i], v, 0.1, `LOGO_END ink ${i}`));
  near(logoScale(LOGO_END) * 10, 39.3, 0.015, "LOGO_END stroke");
  // The sparkle lands on the point: open (1025, 465), end (1522, 432).
  const so = logoToPx(LOGO_OPEN, ...LOGO_POINT);
  near(so.x, 1025, 1e-9, "open sparkle x");
  near(so.y, 465, 1e-9, "open sparkle y");
  const se = logoToPx(LOGO_END, ...LOGO_POINT);
  near(se.x, 1522, 0.2, "end sparkle x");
  near(se.y, 432, 0.2, "end sparkle y");
  // open's docked hook: eye (1090, 420), bowl (1057.5, 480).
  const dock = hookAnchors(LOGO_OPEN, 0);
  assert.deepEqual([dock.eye.x, dock.eye.y, dock.bowl.x, dock.bowl.y], [1090, 420, 1057.5, 480]);
  // Frame 0: the hook 300 px up, its eye at (1090, 120).
  const hanging = hookAnchors(LOGO_OPEN, 0, { offset: [0, -300] });
  assert.deepEqual([hanging.eye.x, hanging.eye.y], [1090, 120]);
  // morph's capsule targets at LOGO_END (storyboard §6.10).
  const seg = (x0: number, y0: number, x1: number, y1: number) => {
    const a = logoToPx(LOGO_END, x0, y0);
    const z = logoToPx(LOGO_END, x1, y1);
    return [a.x, a.y, z.x, z.y];
  };
  const targets: [number[], number[]][] = [
    [seg(28, 16, 28, 80), [1215.7, 157.1, 1215.7, 408.6]],
    [seg(92, 16, 92, 80), [1467.1, 157.1, 1467.1, 408.6]],
    [seg(92, 68, 126, 46), [1467.1, 361.4, 1600.7, 275.0]],
    [seg(100, 62, 122, 76), [1498.6, 337.9, 1585.0, 392.9]],
  ];
  targets.forEach(([got, want], i) => got.forEach((v, j) => near(v, want[j], 0.06, `capsule ${i} coordinate ${j}`)));
});

test("a swung point turns about the leg's root, and a turned hook about its eye", () => {
  const root = logoToPx(LOGO_OPEN, 100, 63, 30);
  assert.deepEqual([root.x, root.y], [1010, 407.5]);
  // Clockwise on screen: the point, below the pivot, swings left for a positive swing.
  const p = logoToPx(LOGO_OPEN, ...LOGO_POINT, 8);
  assert.ok(p.x < 1025 && Math.abs(Math.hypot(p.x - 1010, p.y - 407.5) - Math.hypot(15, 57.5)) < 1e-9);
  const place = hookPlace(1240, 400, 3);
  const a = hookAnchors(place, 0);
  assert.deepEqual([a.eye.x, a.eye.y], [1240, 400]);
  near(a.ringTop.y, 400 - 7.7 * 3, 1e-9, "the ring's top");
  near(a.bottom.y - a.bowl.y, 13 * 3, 1e-9, "the bowl's radius");
  const turned = hookAnchors(place, 0, { rotate: 20 });
  assert.deepEqual([turned.eye.x, turned.eye.y], [1240, 400]);
  near(Math.hypot(turned.bowl.x - 1240, turned.bowl.y - 400), Math.hypot(13, 24) * 3, 1e-9, "the bowl keeps its distance");
  assert.ok(turned.bowl.x < a.bowl.x, "a clockwise turn swings the bowl left");
  near(hookAnchors(place, 1).point.y - hookAnchors(place, 0).point.y, 2 * 3, 1e-9, "the docked point sits 2 units lower");
});

test("penStrokes draws each stroke in its window at a constant pen speed", () => {
  assert.deepEqual(penStrokes(0), [0, 0, 0, 0, 0, 0]);
  assert.deepEqual(penStrokes(1), [1, 1, 1, 1, 1, 1]);
  // identity §1.3's windows, 391 units of pen travel with 10-unit lifts.
  const windows = [
    [0, 0.164],
    [0.189, 0.374],
    [0.399, 0.563],
    [0.588, 0.692],
    [0.717, 0.954],
    [0.98, 1],
  ];
  windows.forEach(([from, to], i) => {
    assert.equal(penStrokes(from - 0.001)[i], 0, `stroke ${i} not started before ${from}`);
    assert.ok(penStrokes(from + 0.002)[i] > 0, `stroke ${i} started by ${from}`);
    assert.ok(penStrokes(to - 0.002)[i] < 1, `stroke ${i} not done before ${to}`);
    assert.equal(penStrokes(Math.min(1, to + 0.001))[i], 1, `stroke ${i} done by ${to}`);
  });
  // Monotonic: the pen never backs up.
  let last = penStrokes(0);
  for (let u = 0.01; u <= 1; u += 0.01) {
    const p = penStrokes(u);
    p.forEach((v, i) => assert.ok(v >= last[i]));
    last = p;
  }
});

test("swingDeg is a pendulum on the beat, damped by tau", () => {
  const [t0, A, tau] = [3, 8, BAR];
  for (const t of [0, 2.9, 3]) assert.equal(swingDeg(t, t0, A, tau), 0);
  // It crosses zero on every half beat and peaks near every odd quarter.
  for (let n = 1; n <= 8; n++) near(swingDeg(t0 + (n * BEAT) / 2, t0, A, tau), 0, 1e-9, `half beat ${n}`);
  assert.ok(swingDeg(t0 + BEAT / 4, t0, A, tau) > 0, "the first swing is positive");
  assert.ok(swingDeg(t0 + (3 * BEAT) / 4, t0, A, tau) < 0, "then it swings back");
  near(swingDeg(t0 + BEAT / 4, t0, A, tau), A * Math.exp(-BEAT / 4 / tau), 1e-9, "the first apex");
  for (let t = t0; t < t0 + 3 * BAR; t += 0.01) {
    assert.ok(Math.abs(swingDeg(t, t0, A, tau)) <= A * Math.exp(-(t - t0) / tau) + 1e-12, `inside its envelope at ${t}`);
  }
});

test("settledSwing eases the swing out and is exactly 0 from its calm end", () => {
  const cases = [
    // open: swingDeg(t, b4, 8°, 1 bar) × (1 − smoothstep(b5.5, b7)).
    { t0: b(4), A: 8, tau: BAR, calm: [b(5.5), b(7)] as const, until: sec("open").end - sec("open").start },
    // end: swingDeg(t, b0, 6°, 1 beat) × (1 − smoothstep(b2, b3)), and the b8 reaction by b10.
    { t0: b(0), A: 6, tau: BEAT, calm: [b(2), b(3)] as const, until: b(8) },
    { t0: b(8), A: 3, tau: BEAT, calm: [b(9), b(10)] as const, until: sec("end").end - sec("end").start },
  ];
  for (const { t0, A, tau, calm, until } of cases) {
    for (let t = t0 - 0.2; t < until; t += 1 / 240) {
      const got = settledSwing(t, t0, A, tau, calm);
      const free = swingDeg(t, t0, A, tau);
      if (t >= calm[1]) assert.ok(Object.is(got, 0), `exactly 0 at ${t}, not ${got}`);
      else if (t <= calm[0]) assert.equal(got, free);
      else assert.ok(Math.abs(got) <= Math.abs(free) + 1e-12);
    }
    assert.ok(Object.is(settledSwing(calm[1], t0, A, tau, calm), 0));
  }
});

// The Chromium half. The page gets the marks as `Logo` and these helpers as `T`.
const HELPERS = String.raw`
window.T = {
  canvas(w, h) {
    const c = document.createElement("canvas");
    c.width = w;
    c.height = h;
    return c.getContext("2d");
  },
  async image(type, b64) {
    const img = new Image();
    img.src = "data:" + type + ";base64," + b64;
    await img.decode();
    return img;
  },
  // Premultiplied, so a transparent pixel's colour does not count.
  diff(A, B, i) {
    let d = Math.abs(A[i + 3] - B[i + 3]);
    for (let c = 0; c < 3; c++) d = Math.max(d, Math.abs((A[i + c] * A[i + 3] - B[i + c] * B[i + 3]) / 255));
    return d;
  },
  // Away from every edge: the reference's 3x3 neighbourhood is one colour.
  inside(A, w, h, x, y) {
    const k = (y * w + x) * 4;
    for (let dy = -1; dy <= 1; dy++)
      for (let dx = -1; dx <= 1; dx++) {
        const X = x + dx, Y = y + dy;
        if (X < 0 || Y < 0 || X >= w || Y >= h) continue;
        const j = (Y * w + X) * 4;
        for (let c = 0; c < 4; c++) if (A[j + c] !== A[k + c]) return false;
      }
    return true;
  },
  compare(a, b) {
    const w = a.canvas.width, h = a.canvas.height;
    const A = a.getImageData(0, 0, w, h).data, B = b.getImageData(0, 0, w, h).data;
    let edge = 0, interior = 0, over = 0, sq = 0, clear = 0;
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        const i = (y * w + x) * 4;
        const d = T.diff(A, B, i);
        sq += d * d;
        if (d > 2) over++;
        if (T.inside(A, w, h, x, y)) {
          interior = Math.max(interior, d);
          // Ink where the reference is clear all around: something stands proud.
          if (A[i + 3] === 0) clear = Math.max(clear, B[i + 3]);
        } else edge = Math.max(edge, d);
      }
    const mse = sq / (w * h);
    return { edge, interior, over, clear, psnr: mse ? 10 * Math.log10((255 * 255) / mse) : 1e9 };
  },
  // The alpha levels of pixels away from every edge.
  levels(a) {
    const w = a.canvas.width, h = a.canvas.height;
    const A = a.getImageData(0, 0, w, h).data;
    const seen = new Set();
    for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) if (T.inside(A, w, h, x, y)) seen.add(A[(y * w + x) * 4 + 3]);
    return [...seen].sort((p, q) => p - q);
  },
  blank(a) {
    const A = a.getImageData(0, 0, a.canvas.width, a.canvas.height).data;
    for (let i = 3; i < A.length; i += 4) if (A[i]) return false;
    return true;
  },
  bbox(a) {
    const w = a.canvas.width, h = a.canvas.height;
    const A = a.getImageData(0, 0, w, h).data;
    let x0 = w, y0 = h, x1 = -1, y1 = -1;
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++)
        if (A[(y * w + x) * 4 + 3] > 8) { x0 = Math.min(x0, x); y0 = Math.min(y0, y); x1 = Math.max(x1, x + 1); y1 = Math.max(y1, y + 1); }
    return { x0, y0, x1, y1 };
  },
};`;

/** Largest difference an edge pixel may show, in 8-bit levels of premultiplied colour or alpha. */
const EDGE = 32;
/** Largest difference a pixel away from every edge may show. */
const INSIDE = 1;

// From docs/, where the browser and bundler are installed. Loaded at run
// time, not bundled into the test.
const load = createRequire(join(REPO, "docs/package.json"));

test("in Chromium, the marks draw as logo.svg and the favicon do", async (t) => {
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
      stdin: { contents: `export * from "./kit/logo";`, resolveDir: SHOWREEL, loader: "ts" },
      bundle: true,
      format: "iife",
      globalName: "Logo",
      target: "es2022",
      write: false,
      logLevel: "error",
    });
    const page = await browser.newPage();
    await page.setContent("<body></body>");
    await page.addScriptTag({ content: bundle.outputFiles[0].text });
    await page.addScriptTag({ content: HELPERS });
    const svg = Buffer.from(SVG).toString("base64");

    await t.test("drawLogo at full progress is logo.svg, at any size and output scale", async () => {
      // [what, canvas w, h, output scale, place, where the viewBox lands in px, alpha, via ctx.globalAlpha]
      const cases = [
        ["LOGO_OPEN", 1920, 1080, 1, LOGO_OPEN, [785, 260, 350, 280], 1, false],
        ["LOGO_END", 1920, 1080, 1, LOGO_END, [1145, 110, 550, 440], 1, false],
        ["1 px a unit", 140, 112, 1, { cx: 70, cy: 56, h: 112 }, [0, 0, 140, 112], 1, false],
        ["LOGO_OPEN at half output scale", 960, 540, 0.5, LOGO_OPEN, [392.5, 130, 175, 140], 1, false],
        ["LOGO_OPEN at alpha 0.5", 1920, 1080, 1, LOGO_OPEN, [785, 260, 350, 280], 0.5, false],
        ["LOGO_OPEN under globalAlpha 0.5", 1920, 1080, 1, LOGO_OPEN, [785, 260, 350, 280], 0.5, true],
      ] as const;
      for (const [what, w, h, scale, place, [x, y, dw, dh], alpha, viaCtx] of cases) {
        const r = await page.evaluate(
          async ({ svg, w, h, scale, place, x, y, dw, dh, alpha, viaCtx }) => {
            const img = await T.image("image/svg+xml", svg);
            const want = T.canvas(w, h);
            want.globalAlpha = alpha;
            want.drawImage(img, x, y, dw, dh);
            const got = T.canvas(w, h);
            got.scale(scale, scale);
            if (viaCtx) got.globalAlpha = alpha;
            Logo.drawLogo(got, place, Logo.LOGO_FULL, { alpha: viaCtx ? 1 : alpha });
            return T.compare(want, got);
          },
          { svg, w, h, scale, place, x, y, dw, dh, alpha, viaCtx },
        );
        t.diagnostic(`${what}: edge max ${r.edge.toFixed(1)}, interior max ${r.interior.toFixed(1)}, ${r.over} px over 2 levels, PSNR ${r.psnr.toFixed(1)} dB`);
        assert.ok(r.interior <= INSIDE, `${what}: a pixel away from any edge differs by ${r.interior}`);
        assert.ok(r.edge <= EDGE, `${what}: an edge pixel differs by ${r.edge}`);
      }
    });

    await t.test("nothing at progress 0, and a partial stroke stops where its dash does", async () => {
      const r = await page.evaluate(() => {
        const c = T.canvas(600, 500);
        const place = { cx: 300, cy: 240, h: 448 };
        Logo.drawLogo(c, place, [0, 0, 0, 0, 0, 0]);
        Logo.drawLogo(c, place, [-1, 0, 0, 0, 0, 0]);
        Logo.drawLogo(c, place, Logo.LOGO_FULL, { alpha: 0 });
        Logo.glint(c, place, 0);
        Logo.glint(c, place, 1);
        Logo.sparkle(c, 300, 240, 0);
        Logo.sparkle(c, 300, 240, 1);
        Logo.drawIconHook(c, place, 0, { alpha: 0 });
        const blank = T.blank(c);
        const half = T.canvas(600, 500);
        Logo.drawLogo(half, place, [0.5, 0, 0, 0, 0, 0]);
        return { blank, box: T.bbox(half) };
      });
      assert.ok(r.blank, "a stroke at 0, a glint or sparkle at 0 or 1, and alpha 0 paint nothing");
      // Half the h stem: x 23–33, y 11 to 16 + 32 + 5 = 53 units, at 4 px a unit.
      const px = (x: number, y: number) => logoToPx({ cx: 300, cy: 240, h: 448 }, x, y);
      const [a, z] = [px(23, 11), px(33, 53)];
      for (const [got, want, what] of [
        [r.box.x0, a.x, "left"],
        [r.box.y0, a.y, "top"],
        [r.box.x1, z.x, "right"],
        [r.box.y1, z.y, "bottom"],
      ] as const)
        near(got, want, 1, `half the h stem's ${what}`);
    });

    await t.test("a docked hook, and a glint, stay inside the wordmark", async () => {
      const cases = [
        ["docked hook", 0],
        ["glint", 0],
        ["glint on a swung leg", 8],
      ] as const;
      for (const [what, swing] of cases) {
        const r = await page.evaluate(
          ({ what, swing }) => {
            const want = T.canvas(1920, 1080);
            Logo.drawLogo(want, Logo.LOGO_OPEN, Logo.LOGO_FULL, { swing });
            const got = T.canvas(1920, 1080);
            Logo.drawLogo(got, Logo.LOGO_OPEN, Logo.LOGO_FULL, { swing });
            if (what === "docked hook") Logo.drawIconHook(got, Logo.LOGO_OPEN, 1);
            else for (const u of [0.02, 0.1, 0.3, 0.6, 0.9, 0.99]) Logo.glint(got, Logo.LOGO_OPEN, u, { swing });
            return T.compare(want, got);
          },
          { what, swing },
        );
        t.diagnostic(`${what}: ink ${r.clear} where the wordmark is clear, interior max ${r.interior}, edge max ${r.edge}`);
        assert.equal(r.clear, 0, `${what} paints outside the wordmark`);
        if (what === "docked hook") {
          assert.ok(r.interior <= INSIDE, `the docked hook shows inside the wordmark: ${r.interior}`);
          assert.ok(r.edge <= 96, `the docked hook shows along the wordmark's edge: ${r.edge}`);
        }
      }
    });

    await t.test("the free hook is the favicon's", async () => {
      const icon = readFileSync(join(REPO, "docs/public/android-chrome-512x512.png")).toString("base64");
      const iou = await page.evaluate(async (icon) => {
        const img = await T.image("image/png", icon);
        const want = T.canvas(512, 512);
        want.drawImage(img, 0, 0);
        const got = T.canvas(512, 512);
        got.fillStyle = "#071019";
        got.fillRect(0, 0, 512, 512);
        // The favicon's bowl: centre (231.75, 319.75), 8 px a logo unit (identity §1.4).
        Logo.drawIconHook(got, { cx: 231.75 + (80 - 119) * 8, cy: 319.75 + (60 - 92) * 8, h: 112 * 8 }, 0);
        const A = want.getImageData(0, 0, 512, 512).data;
        const B = got.getImageData(0, 0, 512, 512).data;
        // Ink coverage from the green channel: #071019's 16 to #4adef0's 222.
        const cov = (D: Uint8ClampedArray, i: number) => Math.max(0, Math.min(1, (D[i + 1] - 16) / 206));
        let both = 0;
        let either = 0;
        for (let i = 0; i < A.length; i += 4) {
          both += Math.min(cov(A, i), cov(B, i));
          either += Math.max(cov(A, i), cov(B, i));
        }
        return both / either;
      }, icon);
      t.diagnostic(`drawIconHook(k 0) vs android-chrome-512x512.png: ink IoU ${iou.toFixed(4)}`);
      assert.ok(iou >= 0.98, `the free hook's ink overlaps the favicon's by only ${iou}`);
    });

    await t.test("a fading eye, and a hook or wordmark at partial alpha, never double up", async () => {
      const r = await page.evaluate(() => {
        const one = T.canvas(700, 700);
        Logo.drawIconHook(one, Logo.hookPlace(350, 150, 8), 0.5, { line: { topY: 0 } });
        const half = T.canvas(700, 700);
        Logo.drawIconHook(half, Logo.hookPlace(350, 150, 8), 0.5, { alpha: 0.5 });
        const logo = T.canvas(700, 600);
        Logo.drawLogo(logo, { cx: 350, cy: 300, h: 560 }, [1, 1, 1, 0.7, 0.5, 0], { alpha: 0.5, swing: 5 });
        return { one: T.levels(one), half: T.levels(half), logo: T.levels(logo) };
      });
      const only = (levels: number[], allowed: number[], what: string) =>
        assert.ok(
          levels.every((v) => allowed.some((a) => Math.abs(v - a) <= 1)),
          `${what}: flat alpha levels ${levels.join(", ")}, expected only ${allowed.join(", ")}`,
        );
      only(r.one, [0, 128, 255], "the hook half docked");
      only(r.half, [0, 64, 128], "the hook half docked at alpha 0.5");
      only(r.logo, [0, 128], "the wordmark at alpha 0.5");
    });

    await t.test("Chromium measures the paths at the hard-coded lengths", async () => {
      const lengths = await page.evaluate((ds) => {
        const svgNs = "http://www.w3.org/2000/svg";
        return ds.map((d) => {
          const p = document.createElementNS(svgNs, "path");
          p.setAttribute("d", d);
          return p.getTotalLength();
        });
      }, LOGO_STROKES.map((s) => s.d));
      LOGO_STROKES.forEach((s, i) => {
        t.diagnostic(`${s.id}: ${lengths[i].toFixed(4)} (hard-coded ${s.len})`);
        near(lengths[i], s.len, 0.01, s.id);
      });
    });
  } finally {
    await browser.close();
  }
});

declare const Logo: typeof import("../kit/logo");
declare const T: {
  canvas(w: number, h: number): CanvasRenderingContext2D;
  image(type: string, b64: string): Promise<HTMLImageElement>;
  compare(a: CanvasRenderingContext2D, b: CanvasRenderingContext2D): { edge: number; interior: number; over: number; clear: number; psnr: number };
  levels(a: CanvasRenderingContext2D): number[];
  blank(a: CanvasRenderingContext2D): boolean;
  bbox(a: CanvasRenderingContext2D): { x0: number; y0: number; x1: number; y1: number };
};
