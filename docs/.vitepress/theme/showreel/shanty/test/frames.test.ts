// The music video drawn in Chromium with the bundled fonts, as the renderer
// draws it (shanty-video.mjs): every line of the words fits the stage, every
// frame at 60 fps draws without an error (a scene played on the song's
// clock is drawn at times its own 120 fps grid never reaches), and where the
// outro hands the mark to the end card, the two meet on the same frame.
// Skipped when no Chromium is installed, unless SHOWREEL_REQUIRE_CHROMIUM
// is set, as the showreel's Chromium tests are.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { test } from "node:test";
import { REPO, SHOWREEL } from "../../test/repo";
import { SONG } from "../song";

// From docs/, where the browser and bundler are installed, at run time.
const load = createRequire(join(REPO, "docs/package.json"));

/** The frame rate the video is rendered at. */
const FPS = 60;
/**
 * Largest difference a pixel may show between two frames that must meet, in
 * 8-bit levels: a quarter of the mark's contrast with the night, which the
 * antialiasing of a stroke's edge moved a fraction of a pixel stays under.
 */
const EDGE = 64;

const BUNDLE = `export { createVideo, TIMED } from "./shanty/video";
export { CROSS } from "./shanty/outro";
export { ANSWER, CALL, LYRIC_WIDTH, lineWidth } from "./shanty/lyrics";
export { reelScene, drawScene } from "./shanty/shot";
export { resetTypeCache } from "./type";`;

/** Chromium, or null (and the test skipped) when there is none to launch. */
async function launch(t: import("node:test").TestContext): Promise<import("playwright-core").Browser | null> {
  const { chromium } = load("playwright-core") as typeof import("playwright-core");
  try {
    return await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined });
  } catch (err) {
    if (process.env.SHOWREEL_REQUIRE_CHROMIUM) throw err;
    t.skip(`no Chromium to draw in: ${String(err).split("\n")[0]}`);
    return null;
  }
}

/** A page with the video bundled in and the fonts loaded as the renderer loads them. */
async function open(browser: import("playwright-core").Browser) {
  const { build } = load("esbuild") as typeof import("esbuild");
  const bundle = await build({
    stdin: { contents: BUNDLE, resolveDir: SHOWREEL, loader: "ts" },
    bundle: true,
    format: "iife",
    globalName: "Shanty",
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
    Shanty.resetTypeCache();
  }, fonts);
  assert.deepEqual(errors, [], "the video loads without an error");
  return { page, errors };
}

test("in Chromium, every line of the words fits the stage", async (t) => {
  const browser = await launch(t);
  if (!browser) return;
  try {
    const { page } = await open(browser);
    const wide = await page.evaluate(() => {
      const ctx = document.createElement("canvas").getContext("2d")!;
      const out: string[] = [];
      for (const l of Shanty.TIMED) {
        for (const [row, style] of [[l.call, Shanty.CALL], [l.answer, Shanty.ANSWER]] as const) {
          if (!row) continue;
          const w = Shanty.lineWidth(ctx, row.text, style);
          if (w > Shanty.LYRIC_WIDTH) out.push(`${l.id}: "${row.text}" is ${Math.round(w)} px`);
        }
      }
      return out;
    });
    assert.deepEqual(wide, [], "wider than the stage");
  } finally {
    await browser.close();
  }
});

test("in Chromium, every frame at 60 fps draws without an error", async (t) => {
  const browser = await launch(t);
  if (!browser) return;
  try {
    // Four pages draw every fourth frame each, on a small canvas: a frame is
    // drawn in the same logical units at any size.
    const PAGES = 4;
    const pages = await Promise.all(Array.from({ length: PAGES }, () => open(browser)));
    const failures = await Promise.all(
      pages.map(({ page }, k) =>
        page.evaluate(
          ({ fps, duration, k, n }) => {
            const c = document.createElement("canvas");
            c.width = 240;
            c.height = 135;
            const ctx = c.getContext("2d", { alpha: false })!;
            const video = Shanty.createVideo();
            const out: string[] = [];
            for (let i = k; i < Math.round(duration * fps); i += n) {
              try {
                video.render(ctx, i / fps, 240, 135);
              } catch (err) {
                if (out.push(`${(i / fps).toFixed(3)} s: ${(err as Error).message}`) >= 10) break;
              }
            }
            return out;
          },
          { fps: FPS, duration: SONG.duration, k, n: PAGES },
        ),
      ),
    );
    assert.deepEqual(failures.flat(), []);
    assert.deepEqual(pages.flatMap((p) => p.errors), []);
  } finally {
    await browser.close();
  }
});

test("in Chromium, the outro's mark reaches the end card's place on the card's own first frame", async (t) => {
  const browser = await launch(t);
  if (!browser) return;
  try {
    const { page } = await open(browser);
    const max = await page.evaluate(
      ({ fps }) => {
        const frame = (draw: (ctx: CanvasRenderingContext2D) => void) => {
          const c = document.createElement("canvas");
          c.width = 1920;
          c.height = 1080;
          const ctx = c.getContext("2d", { alpha: false })!;
          draw(ctx);
          return ctx.getImageData(0, 0, 1920, 1080).data;
        };
        const video = Shanty.createVideo({ raw: true });
        // The crossing's last frame, and the card's first (end.ts: morph|end).
        const got = frame((ctx) => video.render(ctx, Shanty.CROSS[1] - 1 / fps, 1920, 1080));
        const want = frame((ctx) => Shanty.drawScene(ctx, Shanty.reelScene("end"), 0, { W: 1920, H: 1080, t: 0, facts: null }));
        let max = 0;
        for (let i = 0; i < got.length; i += 4) {
          max = Math.max(max, Math.abs(got[i] - want[i]), Math.abs(got[i + 1] - want[i + 1]), Math.abs(got[i + 2] - want[i + 2]));
        }
        return max;
      },
      { fps: FPS },
    );
    // A sixtieth before the card, the mark is a fraction of a pixel short of
    // its place: the strokes' edges shade a little differently, and nothing
    // else. A mark out of place, or a barb that was not drawn back, would
    // differ by the mark's whole contrast with the night (over 200 levels).
    assert.ok(max <= EDGE, `a pixel differs by ${max} levels`);
  } finally {
    await browser.close();
  }
});

// The page's bundle, as the tests above export it into the page.
declare const Shanty: typeof import("../video") &
  typeof import("../outro") &
  typeof import("../lyrics") &
  typeof import("../shot") &
  typeof import("../../type");
