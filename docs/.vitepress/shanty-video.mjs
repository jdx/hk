// Renders the music video for the sea shanty, "Bound for the Main", to
// docs/public/bound-for-the-main.mp4 with its poster frame in
// docs/public/bound-for-the-main-poster.jpg; the shanty page plays it. Like
// the showreel (showreel-video.mjs, whose pipeline this follows), every
// frame is a pure function of time (theme/showreel/shanty/video.ts), drawn
// in headless Chromium with the bundled fonts and piped to ffmpeg; the
// soundtrack is the song itself, docs/public/bound-for-the-main.mp3. The
// docs deploy runs this (`mise run docs:shanty`) before building; local
// builds leave the video out unless it has been rendered.
//
// Needs ffmpeg on PATH and Playwright's Chromium headless shell
// (`aube exec playwright-core install chromium-headless-shell` in docs/), or
// a Chromium-based browser at CHROME_PATH.
//
// Drafts and stills, for watching a change without touching the published
// files (a full render takes minutes):
//
//   node .vitepress/shanty-video.mjs --out <file.mp4> [--fps 30|60] [--from <s>] [--until <s>]
//   node .vitepress/shanty-video.mjs --stills <dir> (--times 12.5,40 | --every <s>) [--from <s>] [--until <s>]
//        [--scale 0.5] [--sheet] [--raw]
//
//   --out <file.mp4>   one video, at --fps (default 60), to this file only
//   --from, --until    the stretch of the song to draw, seconds
//   --stills <dir>     PNG frames instead, named by time and shot, and with
//                      --sheet a contact sheet of them; --raw leaves out the
//                      words, grain and vignette
//
// With no arguments it makes the full render. The outputs of a draft must be
// outside docs/public, so only a full render ever replaces the files the
// site deploys.

import { once } from "node:events";
import { spawn } from "node:child_process";
import { mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism } from "node:os";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright-core";

const here = dirname(fileURLToPath(import.meta.url));
const PUBLIC = resolve(here, "../public");
const SONG = join(PUBLIC, "bound-for-the-main.mp3");
const VIDEO = join(PUBLIC, "bound-for-the-main.mp4");
const POSTER = join(PUBLIC, "bound-for-the-main-poster.jpg");
// Written outside public/ and renamed over the outputs only once ffmpeg
// succeeds, so a failed render never leaves a partial file to publish.
const staging = resolve(here, "cache/shanty");
const WIDTH = 1920;
const HEIGHT = 1080;
// See showreel-video.mjs: two to four pages keep x264 fed.
const PAGES = Math.max(1, Math.min(4, availableParallelism() >> 1));
const DEPTH = 2;
const FONTS = [
  { file: "SpaceGrotesk.ttf", family: "Space Grotesk", weight: "300 700" },
  { file: "LiberationMono-Regular.ttf", family: "Liberation Mono", weight: "400" },
  { file: "LiberationMono-Bold.ttf", family: "Liberation Mono", weight: "700" },
];

/**
 * A request that cannot be drawn once the video is loaded: thrown so the
 * browser and any encoder are closed first, then reported as fail() would.
 */
class Refusal extends Error {}

function fail(message) {
  console.error(`shanty-video: ${message}`);
  process.exit(2);
}

function parseArgs(argv) {
  const o = { scale: 1, raw: false, sheet: false };
  const value = (i, flag) => {
    const v = argv[i + 1];
    if (v === undefined || v.startsWith("--")) fail(`${flag} needs a value`);
    return v;
  };
  const number = (v, flag) => {
    const n = Number(v);
    if (v.trim() === "" || !Number.isFinite(n)) fail(`${flag}: "${v}" is not a number`);
    return n;
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    switch (a) {
      case "--out": o.out = resolve(value(i++, a)); break;
      case "--stills": o.stills = resolve(value(i++, a)); break;
      case "--fps": o.fps = number(value(i++, a), a); break;
      case "--from": o.from = number(value(i++, a), a); break;
      case "--until": o.until = number(value(i++, a), a); break;
      case "--times": o.times = value(i++, a).split(",").map((v) => number(v, a)); break;
      case "--every": o.every = number(value(i++, a), a); break;
      case "--scale": o.scale = number(value(i++, a), a); break;
      case "--sheet": o.sheet = true; break;
      case "--raw": o.raw = true; break;
      case "-h":
      case "--help": {
        const head = readFileSync(fileURLToPath(import.meta.url), "utf8").split("\n\nimport")[0];
        console.log(head.replace(/^\/\/ ?/gm, ""));
        process.exit(0);
      }
      default: fail(`unknown argument "${a}" (see --help)`);
    }
  }
  if (o.out && o.stills) fail("pass one of --out and --stills");
  const draft = Boolean(o.out || o.stills);
  if (!draft && (o.fps !== undefined || o.from !== undefined || o.until !== undefined)) {
    fail("--fps, --from and --until make a draft, which needs --out <file.mp4> or --stills <dir>");
  }
  if ((o.times || o.every || o.sheet || o.raw || o.scale !== 1) && !o.stills) fail("--times, --every, --sheet, --raw and --scale are for --stills");
  if (o.stills && !o.times && !o.every) fail("--stills needs --times or --every");
  if (o.fps !== undefined && ![30, 60].includes(o.fps)) fail("--fps must be 30 or 60");
  if (o.every !== undefined && !(o.every > 0)) fail("--every must be positive");
  if (!(o.scale > 0 && o.scale <= 2)) fail("--scale must be in (0, 2]");
  if (o.out && !/\.mp4$/i.test(o.out)) fail("--out names the draft's .mp4 file");
  for (const target of [o.out, o.stills].filter(Boolean)) {
    const rel = relative(PUBLIC, target);
    const outside = rel === ".." || rel.startsWith(`..${sep}`) || isAbsolute(rel);
    if (!outside) fail(`a draft is never written to ${PUBLIC}, which the site deploys; run without arguments for the full render`);
  }
  return o;
}

const opts = parseArgs(process.argv.slice(2));
const draft = Boolean(opts.out || opts.stills);

const bundle = await build({
  stdin: {
    contents: `export { createVideo, POSTER_TIME, resetTypeCache, shotAt } from "./theme/showreel/shanty/video.ts";`,
    resolveDir: here,
    loader: "ts",
  },
  bundle: true,
  format: "iife",
  globalName: "Shanty",
  target: "es2022",
  write: false,
  logLevel: "error",
});

const started = performance.now();
const scale = opts.stills ? opts.scale : 1;
const pw = Math.round(WIDTH * scale);
const ph = Math.round(HEIGHT * scale);
const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined });
const partials = [];
let encoder = null;
let refused = null;
try {
  let pageError = null;
  const fonts = FONTS.map((font) => ({ ...font, bytes: readFileSync(resolve(here, "fonts", font.file)).toString("base64") }));
  const pages = await Promise.all(
    Array.from({ length: PAGES }, async () => {
      const page = await browser.newPage();
      page.on("pageerror", (err) => {
        pageError ??= err;
      });
      await page.setContent(`<canvas id="reel" width="${pw}" height="${ph}"></canvas>`);
      await page.addScriptTag({ content: bundle.outputFiles[0].text });
      await page.evaluate(
        async ({ fonts, raw }) => {
          for (const font of fonts) {
            const bytes = Uint8Array.from(atob(font.bytes), (c) => c.charCodeAt(0));
            document.fonts.add(await new FontFace(font.family, bytes, { weight: font.weight }).load());
          }
          // Measurements cached before the fonts loaded would be a fallback's.
          Shanty.resetTypeCache();
          window.video = Shanty.createVideo({ raw });
          window.canvas = document.getElementById("reel");
          window.ctx = window.canvas.getContext("2d", { alpha: false });
        },
        { fonts, raw: opts.raw },
      );
      return page;
    }),
  );
  if (pageError) throw pageError;
  const [page] = pages;
  const full = await page.evaluate(() => window.video.duration);
  const from = Math.max(0, opts.from ?? 0);
  const until = Math.min(opts.until ?? full, full);
  if (!(until > from)) throw new Refusal(`nothing to draw from ${from} to ${until} s (the video is ${full} s)`);

  /** Draw frame `t` on page `p`, as JPEG or PNG. */
  const frame = (p, t, type, quality) =>
    pages[p % pages.length].evaluate(
      ({ t, pw, ph, type, quality }) => {
        window.video.render(window.ctx, t, pw, ph);
        return window.canvas.toDataURL(type, quality);
      },
      { t, pw, ph, type, quality },
    );
  const bytes = (url) => Buffer.from(url.slice(url.indexOf(",") + 1), "base64");

  if (opts.stills) {
    const times = [...(opts.times ?? [])];
    if (opts.every) for (let t = from; t < until - 1e-9; t += opts.every) times.push(+t.toFixed(4));
    mkdirSync(opts.stills, { recursive: true });
    const shots = await page.evaluate((ts) => ts.map((t) => Shanty.shotAt(t).name), times);
    const written = await Promise.all(
      times.map(async (t, i) => {
        const file = join(opts.stills, `t${t.toFixed(3).padStart(7, "0")}_${shots[i]}.png`);
        writeFileSync(file, bytes(await frame(i, t, "image/png")));
        return file;
      }),
    );
    if (pageError) throw pageError;
    if (opts.sheet) {
      const cols = Math.min(6, times.length);
      const tw = 480;
      const th = 270;
      const png = await page.evaluate(
        async ({ files, labels, cols, tw, th }) => {
          const rows = Math.ceil(files.length / cols);
          const sheet = document.createElement("canvas");
          sheet.width = 8 + cols * (tw + 8);
          sheet.height = 8 + rows * (th + 34);
          const g = sheet.getContext("2d");
          g.fillStyle = "#1a1f24";
          g.fillRect(0, 0, sheet.width, sheet.height);
          for (const [i, src] of files.entries()) {
            const img = new Image();
            img.src = src;
            await img.decode();
            const x = 8 + (i % cols) * (tw + 8);
            const y = 8 + Math.floor(i / cols) * (th + 34);
            g.drawImage(img, x, y, tw, th);
            g.fillStyle = "#adbdc9";
            g.font = '400 18px "Liberation Mono", monospace';
            g.fillText(labels[i], x + 2, y + th + 22);
          }
          return sheet.toDataURL("image/png");
        },
        {
          files: written.map((f) => `data:image/png;base64,${readFileSync(f).toString("base64")}`),
          labels: times.map((t, i) => `${t.toFixed(2)} s · ${shots[i]}`),
          cols,
          tw,
          th,
        },
      );
      const file = join(opts.stills, "sheet.png");
      writeFileSync(file, bytes(png));
      written.push(file);
    }
    for (const file of written) console.log(file);
    console.error(`${times.length} stills in ${((performance.now() - started) / 1000).toFixed(1)} s`);
  } else {
    const fps = opts.fps ?? 60;
    // The frames the window holds, i / fps from `first`; counted before
    // ffmpeg starts, so a window too short to hold one starts no encoder.
    const first = Math.round(from * fps);
    const total = Math.round(until * fps) - first;
    if (total <= 0) throw new Refusal(`no frame at ${fps} fps between ${from} and ${until} s`);
    const out = opts.out ?? VIDEO;
    const partial = opts.out ? join(dirname(out), `.${out.split(sep).pop()}.partial`) : join(staging, "bound-for-the-main.mp4");
    mkdirSync(dirname(partial), { recursive: true });
    partials.push(partial);
    // 1080p H.264 High at 60 fps with the song as AAC, the index up front.
    // Frames arrive as JPEG (BT.601), tagged so players decode them alike.
    const ffmpeg = spawn(
      "ffmpeg",
      [
        "-y", "-loglevel", "error",
        "-f", "image2pipe", "-framerate", String(fps), "-c:v", "mjpeg", "-i", "pipe:0",
        "-ss", String(from), "-t", String(until - from), "-i", SONG,
        "-map", "0:v", "-map", "1:a",
        "-c:v", "libx264", "-preset", "slow", "-crf", "23",
        "-profile:v", "high", "-level:v", "4.2", "-pix_fmt", "yuv420p",
        "-colorspace", "bt470bg",
        "-c:a", "aac", "-b:a", "192k",
        "-movflags", "+faststart", "-shortest",
        "-f", "mp4", partial,
      ],
      { stdio: ["pipe", "inherit", "inherit"] },
    );
    encoder = ffmpeg;
    await once(ffmpeg, "spawn");
    const exited = once(ffmpeg, "close");
    exited.catch(() => {});
    let pipeError = null;
    ffmpeg.stdin.on("error", (err) => {
      pipeError ??= err;
    });
    const pending = new Map();
    let queued = 0;
    for (let i = 0; i < total; i++) {
      for (; queued < Math.min(total, i + pages.length * DEPTH); queued++) {
        const f = frame(queued, (first + queued) / fps, "image/jpeg", 0.95);
        f.catch(() => {});
        pending.set(queued, f);
      }
      const jpeg = await pending.get(i);
      pending.delete(i);
      if (pipeError) throw pipeError;
      if (!ffmpeg.stdin.write(bytes(jpeg))) await once(ffmpeg.stdin, "drain");
      if ((i + 1) % (fps * 10) === 0) console.log(`Rendered ${i + 1} of ${total} frames in ${((performance.now() - started) / 1000).toFixed(0)} s`);
    }
    ffmpeg.stdin.end();
    const [code] = await exited;
    if (code !== 0) throw new Error(`ffmpeg exited with ${code}`);
    if (pageError) throw pageError;
    if (draft) {
      renameSync(partial, out);
      console.log(`Rendered ${total} frames to ${out} in ${((performance.now() - started) / 1000).toFixed(0)} s`);
    } else {
      const posterPartial = join(staging, "bound-for-the-main-poster.jpg");
      partials.push(posterPartial);
      const time = await page.evaluate(() => Shanty.POSTER_TIME);
      writeFileSync(posterPartial, bytes(await frame(0, time, "image/jpeg", 0.9)));
      if (pageError) throw pageError;
      renameSync(partial, VIDEO);
      renameSync(posterPartial, POSTER);
      console.log(`Rendered ${VIDEO} and ${POSTER} in ${((performance.now() - started) / 1000).toFixed(0)} s`);
    }
  }
} catch (err) {
  if (!(err instanceof Refusal)) throw err;
  refused = err;
} finally {
  encoder?.kill("SIGKILL");
  await browser.close();
  for (const p of partials) rmSync(p, { force: true });
}
if (refused) fail(refused.message);
