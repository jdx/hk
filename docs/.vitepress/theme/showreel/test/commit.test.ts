// git commit runs hk (storyboard §6.3, scenes/commit.ts): the hits the score
// imports run in order on the sixteenth grid; the still pane after the files
// ✔ is short and under the caption until just before the bar line; and, in
// Chromium, nothing is drawn in the captions' band while the caption is up,
// and from the ✔'s settling every frame is commit|stash's.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join } from "node:path";
import { test } from "node:test";
import { BEAT, sec } from "../bible";
import { CAPTIONS, COMMIT_BEATS, scene } from "../scenes/commit";
import { timeCaptions } from "../type";
import { factsFor } from "./published";
import { REPO, SHOWREEL } from "./repo";

const S = sec("commit");
/** The files row is at rest this long after its beat (scenes/commit.ts FILES_DONE). */
const FILES_DONE = COMMIT_BEATS.files * BEAT + 0.22;

test("the hits run in order on the sixteenth grid", () => {
  const { git, slam, flips, enter, run, files } = COMMIT_BEATS;
  const hits = [git, slam, ...flips, enter, run, files];
  hits.forEach((h, i) => {
    assert.equal(h * 4, Math.round(h * 4), `hit ${i} (b${h}) is on a sixteenth`);
    if (i) assert.ok(h > hits[i - 1], `hit ${i} (b${h}) comes after b${hits[i - 1]}`);
  });
  assert.ok(files < S.bars * 4, "the files ✔ lands inside the section");
});

test("the still pane after the files ✔ is short and captioned to the bar line", () => {
  // The reel's longest still was 3 s here, its last 0.65 s uncaptioned.
  const still = S.len - FILES_DONE;
  assert.ok(still <= 2.6, `the pane is still for ${still.toFixed(2)} s`);
  const [c] = timeCaptions(S, CAPTIONS);
  assert.ok(c.start >= S.beat(COMMIT_BEATS.enter) - BEAT / 4, "the caption rises with Enter, not over the typing");
  assert.ok(c.end >= S.end - BEAT / 4, `the caption is gone ${(S.end - c.end).toFixed(3)} s before the bar line`);
});

const load = createRequire(join(REPO, "docs/package.json"));

test("in Chromium, the captions' band is empty while the caption is up, and the pane is still from the ✔", async (t) => {
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
export { scene } from "./scenes/commit";
export { resetTypeCache } from "./type";`,
        resolveDir: SHOWREEL,
        loader: "ts",
      },
      bundle: true,
      format: "iife",
      globalName: "Commit",
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
      Commit.resetTypeCache();
    }, fonts);
    // Every 64th while any line of the caption is on screen, from its first word to the end of its wipe.
    const band: number[] = [];
    for (const c of timeCaptions(S, CAPTIONS)) for (let at = c.start; at < c.end; at += BEAT / 16) band.push(at);
    // Every 32nd from the ✔'s settling to the last frame.
    const still: number[] = [];
    for (let at = S.at(FILES_DONE); at < S.end; at += BEAT / 8) still.push(at);
    const last = S.end - 1 / 120;
    const { bad, moved } = await page.evaluate(
      ({ facts, band, still, last }) => {
        const reel = Commit.composeReel([Commit.scene], facts, { raw: true });
        const ctx = document.createElement("canvas").getContext("2d", { alpha: false, willReadFrequently: true })!;
        ctx.canvas.width = 1920;
        ctx.canvas.height = 1080;
        const bad: string[] = [];
        for (const at of band) {
          reel.render(ctx, at, 1920, 1080);
          const d = ctx.getImageData(0, 740, 1920, 226).data;
          // The stage's own colour, bg #0c151d, within a level.
          let over = 0;
          for (let i = 0; i < d.length; i += 4) if (Math.abs(d[i] - 12) > 1 || Math.abs(d[i + 1] - 21) > 1 || Math.abs(d[i + 2] - 29) > 1) over++;
          if (over) bad.push(`${at.toFixed(3)} s: ${over} px`);
        }
        reel.render(ctx, last, 1920, 1080);
        const want = ctx.getImageData(0, 0, 1920, 1080).data;
        const moved: string[] = [];
        for (const at of still) {
          reel.render(ctx, at, 1920, 1080);
          const d = ctx.getImageData(0, 0, 1920, 1080).data;
          let off = 0;
          for (let i = 0; i < d.length; i++) if (Math.abs(d[i] - want[i]) > 1) off++;
          if (off) moved.push(`${at.toFixed(3)} s: ${off} channels`);
        }
        return { bad, moved };
      },
      { facts: factsFor("both"), band, still, last },
    );
    assert.deepEqual(bad, [], "scene pixels in the captions' band while the caption is up");
    assert.deepEqual(moved, [], "the pane moves after the files ✔ has settled");
    assert.deepEqual(errors, [], "no frame throws");
  } finally {
    await browser.close();
  }
});

declare const Commit: typeof import("../compose") & { scene: typeof scene } & typeof import("../type");
