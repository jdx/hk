// Renders the showreel to docs/public/showreel-120.mp4 and showreel.mp4 (the
// same reel at 120 and 60 fps), with its poster frame in
// docs/public/showreel-poster.jpg. The landing page plays the 120 fps file
// where the browser can decode it smoothly and the 60 fps file elsewhere, and
// the homepage offers the 60 fps file as og:video so link previews that play
// video (Discord, iMessage, Telegram) can play it too. Every frame of the reel
// is a pure function of time; the score is rendered offline. The docs deploy
// runs this (`mise run docs:showreel`) before building; local builds leave
// the showreel out unless it has been rendered.
//
// Needs ffmpeg on PATH and Playwright's Chromium headless shell
// (`aube exec playwright-core install chromium-headless-shell` in docs/), or
// a Chromium-based browser at CHROME_PATH.
//
// Drafts, for hearing or watching a change without touching the published
// files (a full render takes minutes):
//
//   node .vitepress/showreel-video.mjs --out <file.mp4> [--fps 60|120] [--until <s>] [--facts <v>]
//   node .vitepress/showreel-video.mjs --audio-only <file.wav> [--until <s>] [--facts <v>]
//
//   --out <file.mp4>         one video, at --fps (default 60), to this file only
//   --fps 60|120             the draft's frame rate
//   --until <seconds>        stop at this reel time (default: the whole reel)
//   --audio-only <file.wav>  only the score, as 48 kHz 16-bit stereo PCM
//   --facts <v>              the benchmark facts: live (benchmark/results.json,
//                            what a full render uses and the default), both,
//                            one or none (the variants showreel-frames.mjs
//                            previews, from test/published.ts)
//
// With no arguments it makes the full render. --fps, --until and a --facts
// other than live need --out or --audio-only, which must name a file outside
// docs/public, so only a full render of the published facts ever replaces
// the files the site deploys.

import { once } from "node:events";
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism, tmpdir } from "node:os";
import { basename, dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright-core";

const here = dirname(fileURLToPath(import.meta.url));
// The site deploys everything in public/.
const PUBLIC = resolve(here, "../public");
const poster = join(PUBLIC, "showreel-poster.jpg");
// Written outside public/ and renamed over the outputs only once ffmpeg
// succeeds, so a failed or interrupted render never leaves a partial file
// for the build to publish. The cache directory is on the same filesystem
// (so the rename is atomic) and is never deployed.
const staging = resolve(here, "cache/showreel");
const posterPartial = join(staging, "showreel-poster.jpg");
const WIDTH = 1920;
const HEIGHT = 1080;
// One pass renders every frame at 120 fps. Every 60 fps frame is also a
// 120 fps frame (i/60 == 2i/120), so the 60 fps file takes every other one.
// 1080p120 needs H.264 level 5.1; the 60 fps file, which link previews play,
// stays at 4.2.
const VIDEOS = [
  { name: "showreel-120.mp4", fps: 120, level: "5.1" },
  { name: "showreel.mp4", fps: 60, level: "4.2" },
];
// Frames do not depend on each other, so pages render them side by side. On
// a 4-vCPU runner x264 is the floor: two pages render about 12% faster than
// one, and a third adds nothing. Bigger machines get up to four.
const PAGES = Math.max(1, Math.min(4, availableParallelism() >> 1));
// Frames each page has queued, so one page hands back a frame while it draws
// the next.
const DEPTH = 2;
// Rendered before the reel starts and trimmed, so the score's compressor
// lookahead can place the first sounds exactly.
const PRE_ROLL = 0.2;
const SAMPLE_RATE = 48000;
const FONTS = [
  { file: "SpaceGrotesk.ttf", family: "Space Grotesk", weight: "300 700" },
  { file: "LiberationMono-Regular.ttf", family: "Liberation Mono", weight: "400" },
  { file: "LiberationMono-Bold.ttf", family: "Liberation Mono", weight: "700" },
];

function fail(message) {
  console.error(`showreel-video: ${message}`);
  process.exit(2);
}

function parseArgs(argv) {
  const o = { facts: "live" };
  const value = (i, flag) => {
    const v = argv[i + 1];
    if (v === undefined || v.startsWith("--")) fail(`${flag} needs a value`);
    return v;
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    switch (a) {
      case "--out": o.out = resolve(value(i++, a)); break;
      case "--audio-only": o.audioOnly = resolve(value(i++, a)); break;
      case "--fps": o.fps = Number(value(i++, a)); break;
      case "--until": o.until = Number(value(i++, a)); break;
      case "--facts": o.facts = value(i++, a); break;
      case "-h":
      case "--help": {
        const head = readFileSync(fileURLToPath(import.meta.url), "utf8").split("\n\nimport")[0];
        console.log(head.replace(/^\/\/ ?/gm, ""));
        process.exit(0);
      }
      default: fail(`unknown argument "${a}" (see --help)`);
    }
  }
  if (o.out && o.audioOnly) fail("pass one of --out and --audio-only");
  if (!o.out && !o.audioOnly && (o.fps !== undefined || o.until !== undefined || o.facts !== "live")) {
    fail("--fps, --until and --facts make a draft, which needs --out <file.mp4> or --audio-only <file.wav>; only a full render writes docs/public");
  }
  if (o.audioOnly && o.fps !== undefined) fail("--fps is for video; --audio-only renders only the score");
  if (o.fps !== undefined && !VIDEOS.some((video) => video.fps === o.fps)) fail(`--fps must be 60 or 120`);
  if (o.until !== undefined && !(o.until > 0)) fail("--until must be a positive number of seconds");
  if (!["live", "both", "one", "none"].includes(o.facts)) fail(`--facts must be live, both, one or none, not "${o.facts}"`);
  if (o.out && !/\.mp4$/i.test(o.out)) fail("--out names the draft's .mp4 file");
  if (o.audioOnly && !/\.wav$/i.test(o.audioOnly)) fail("--audio-only names the score's .wav file");
  const target = o.out ?? o.audioOnly;
  if (target && !relative(PUBLIC, target).startsWith("..")) fail(`a draft is never written to ${PUBLIC}, which the site deploys; run without arguments for the full render`);
  return o;
}

const opts = parseArgs(process.argv.slice(2));
const draft = Boolean(opts.out || opts.audioOnly);

// The same published run the benchmarks page reads. factsFromBenchmarks
// applies the page's validity rules and the reel's own, so a failed,
// unverified or inconclusive run leaves every number out.
function benchmarkResults() {
  try {
    return JSON.parse(readFileSync(resolve(here, "../../benchmark/results.json"), "utf8"));
  } catch {
    return null;
  }
}

/** A facts variant other than the live run, built in node by the tests' own helpers. */
async function variantFacts(variant) {
  const work = mkdtempSync(join(tmpdir(), "showreel-facts-"));
  // test/repo.ts finds the checkout from the working directory.
  const cwd = process.cwd();
  process.chdir(here);
  try {
    const bundled = await build({
      stdin: { contents: `export { factsFor } from "./theme/showreel/test/published.ts";`, resolveDir: here, loader: "ts" },
      bundle: true,
      platform: "node",
      format: "esm",
      target: "node22",
      write: false,
      logLevel: "error",
    });
    const file = join(work, "facts.mjs");
    writeFileSync(file, bundled.outputFiles[0].text);
    const { factsFor } = await import(pathToFileURL(file).href);
    return factsFor(variant);
  } finally {
    process.chdir(cwd);
    rmSync(work, { recursive: true, force: true });
  }
}

const bundle = await build({
  stdin: {
    contents: `export { createReel, factsFromBenchmarks, POSTER_TIME, resetTypeCache } from "./theme/showreel/reel.ts";
export { playScore } from "./theme/showreel/audio.ts";`,
    resolveDir: here,
    loader: "ts",
  },
  bundle: true,
  format: "iife",
  globalName: "Showreel",
  target: "es2022",
  write: false,
  logLevel: "error",
});

/** 16-bit stereo PCM, base64 from the page, as a WAV file. */
function wavFile(pcm) {
  const samples = Buffer.from(pcm, "base64");
  const header = Buffer.alloc(44);
  header.write("RIFF", 0);
  header.writeUInt32LE(36 + samples.length, 4);
  header.write("WAVEfmt ", 8);
  header.writeUInt32LE(16, 16);
  header.writeUInt16LE(1, 20);
  header.writeUInt16LE(2, 22);
  header.writeUInt32LE(SAMPLE_RATE, 24);
  header.writeUInt32LE(SAMPLE_RATE * 4, 28);
  header.writeUInt16LE(4, 32);
  header.writeUInt16LE(16, 34);
  header.write("data", 36);
  header.writeUInt32LE(samples.length, 40);
  return Buffer.concat([header, samples]);
}

// The live run is reduced to facts inside the page, by the bundle the reel
// draws with; a variant arrives already built.
const input = opts.facts === "live" ? { results: benchmarkResults() } : { facts: await variantFacts(opts.facts) };
const fps = opts.fps ?? 60;
const FPS = opts.out ? fps : 120;
const encoders = opts.audioOnly
  ? []
  : opts.out
    ? [
        {
          name: basename(opts.out),
          fps,
          level: VIDEOS.find((video) => video.fps === fps).level,
          out: opts.out,
          // Beside the draft, so the rename cannot cross filesystems.
          partial: join(dirname(opts.out), `.${basename(opts.out)}.partial`),
        },
      ]
    : VIDEOS.map((video) => ({
        ...video,
        out: join(PUBLIC, video.name),
        partial: join(staging, video.name),
      }));

if (!draft) mkdirSync(staging, { recursive: true });
if (opts.out) mkdirSync(dirname(opts.out), { recursive: true });
const started = performance.now();
const browser = await chromium.launch({
  executablePath: process.env.CHROME_PATH || undefined,
});
const work = mkdtempSync(join(tmpdir(), "showreel-"));
try {
  let pageError = null;
  const fonts = FONTS.map((font) => ({ ...font, bytes: readFileSync(resolve(here, "fonts", font.file)).toString("base64") }));
  const pages = await Promise.all(
    Array.from({ length: opts.audioOnly ? 1 : PAGES }, async () => {
      const page = await browser.newPage();
      page.on("pageerror", (err) => {
        pageError ??= err;
      });
      await page.setContent(
        `<canvas id="reel" width="${WIDTH}" height="${HEIGHT}"></canvas>`,
      );
      await page.addScriptTag({ content: bundle.outputFiles[0].text });
      await page.evaluate(
        async ({ fonts, input }) => {
          for (const font of fonts) {
            const bytes = Uint8Array.from(atob(font.bytes), (c) => c.charCodeAt(0));
            const face = new FontFace(font.family, bytes, { weight: font.weight });
            document.fonts.add(await face.load());
          }
          // layout() caches measurements; none may survive from a fallback font.
          Showreel.resetTypeCache();
          const canvas = document.getElementById("reel");
          window.facts = "facts" in input ? input.facts : Showreel.factsFromBenchmarks(input.results);
          window.reel = Showreel.createReel(window.facts);
          window.ctx = canvas.getContext("2d", { alpha: false });
        },
        { fonts, input },
      );
      return page;
    }),
  );
  const [page] = pages;
  // The reel's own length (bible.ts), so video and score follow its timing.
  const full = await page.evaluate(() => window.reel.duration);
  const duration = Math.min(opts.until ?? full, full);
  const claims = await page.evaluate(() =>
    [window.facts?.fixAll, window.facts?.checkAll].filter(Boolean).map((race) => `${race.title} ${race.claim.ratio}× vs ${race.claim.rival.label}`),
  );
  console.log(
    `Benchmark facts (${opts.facts}): ${claims.length ? claims.join("; ") : "no race claims"}; rendering ${duration} of ${full} s`,
  );

  // The score, as 16-bit stereo PCM. It hears the same facts as the picture,
  // so the race's cues land on its bars.
  const pcm = await page.evaluate(
    async ({ duration, preRoll, rate }) => {
      const ac = new OfflineAudioContext(2, Math.ceil(rate * (duration + preRoll)), rate);
      Showreel.playScore(ac, ac.destination, 0, preRoll, window.facts);
      const buffer = await ac.startRendering();
      const skip = Math.round(preRoll * rate);
      const frames = Math.round(duration * rate);
      const left = buffer.getChannelData(0);
      const right = buffer.getChannelData(1);
      const data = new Int16Array(frames * 2);
      for (let i = 0; i < frames; i++) {
        data[i * 2] = Math.max(-1, Math.min(1, left[i + skip])) * 32767;
        data[i * 2 + 1] = Math.max(-1, Math.min(1, right[i + skip])) * 32767;
      }
      let binary = "";
      const bytes = new Uint8Array(data.buffer);
      for (let i = 0; i < bytes.length; i += 0x8000) {
        binary += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
      }
      return btoa(binary);
    },
    { duration, preRoll: PRE_ROLL, rate: SAMPLE_RATE },
  );
  if (pageError) throw pageError;
  if (opts.audioOnly) {
    mkdirSync(dirname(opts.audioOnly), { recursive: true });
    writeFileSync(opts.audioOnly, wavFile(pcm));
    const elapsed = (performance.now() - started) / 1000;
    console.log(`Rendered ${opts.audioOnly} in ${elapsed.toFixed(1)} s`);
  } else {
    const wav = join(work, "score.wav");
    writeFileSync(wav, wavFile(pcm));

    // 1080p H.264 High with AAC and the index up front: sharp on the landing
    // page and, at 60 fps, playable by every link preview that plays video.
    // Frames arrive as JPEG, which is BT.601 YCbCr; the tag makes players
    // decode it with the same matrix.
    for (const encoder of encoders) {
      const ffmpeg = spawn(
        "ffmpeg",
        [
          "-y", "-loglevel", "error",
          "-f", "image2pipe", "-framerate", String(encoder.fps), "-c:v", "mjpeg", "-i", "pipe:0",
          "-i", wav,
          "-c:v", "libx264", "-preset", "slow", "-crf", "23",
          "-profile:v", "high", "-level:v", encoder.level, "-pix_fmt", "yuv420p",
          "-colorspace", "bt470bg",
          "-c:a", "aac", "-b:a", "128k",
          "-movflags", "+faststart", "-shortest",
          // Named by the flag, since a draft's partial file has no .mp4 extension.
          "-f", "mp4", encoder.partial,
        ],
        { stdio: ["pipe", "inherit", "inherit"] },
      );
      encoder.ffmpeg = ffmpeg;
      // Fail here, inside the try, if ffmpeg cannot start at all (not on PATH).
      await once(ffmpeg, "spawn");
      encoder.exited = once(ffmpeg, "close");
      // If ffmpeg dies mid-stream, the next frame rethrows its broken pipe; keep
      // that error and the pending close from escaping the try as unhandled.
      encoder.exited.catch(() => {});
      encoder.pipeError = null;
      ffmpeg.stdin.on("error", (err) => {
        encoder.pipeError ??= err;
      });
    }

    // JPEG at 0.95 rather than PNG: the grain makes every PNG about 2.5 MB, and
    // encoding and moving one takes about three times as long. Pages finish
    // out of order, so frames are held until the ones before them are written.
    const total = Math.round(FPS * duration);
    const capture = (i) =>
      pages[i % pages.length].evaluate(
        ({ t, w, h }) => {
          window.reel.render(window.ctx, t, w, h);
          return document.getElementById("reel").toDataURL("image/jpeg", 0.95);
        },
        { t: i / FPS, w: WIDTH, h: HEIGHT },
      );
    const pending = new Map();
    let queued = 0;
    for (let i = 0; i < total; i++) {
      for (; queued < Math.min(total, i + pages.length * DEPTH); queued++) {
        const frame = capture(queued);
        // Awaited in order below; until then a failure must not go unhandled.
        frame.catch(() => {});
        pending.set(queued, frame);
      }
      const jpeg = await pending.get(i);
      pending.delete(i);
      const frame = Buffer.from(jpeg.slice(jpeg.indexOf(",") + 1), "base64");
      const drained = [];
      for (const encoder of encoders) {
        if (i % (FPS / encoder.fps)) continue;
        if (encoder.pipeError) throw encoder.pipeError;
        if (!encoder.ffmpeg.stdin.write(frame)) drained.push(once(encoder.ffmpeg.stdin, "drain"));
      }
      await Promise.all(drained);
      if ((i + 1) % (FPS * 10) === 0) {
        const elapsed = (performance.now() - started) / 1000;
        console.log(`Rendered ${i + 1} of ${total} frames in ${elapsed.toFixed(0)} s`);
      }
    }
    for (const encoder of encoders) encoder.ffmpeg.stdin.end();
    for (const encoder of encoders) {
      const [code] = await encoder.exited;
      if (code !== 0) throw new Error(`ffmpeg exited with ${code} for ${encoder.name}`);
    }
    if (pageError) throw pageError;

    if (draft) {
      for (const encoder of encoders) renameSync(encoder.partial, encoder.out);
      const elapsed = (performance.now() - started) / 1000;
      console.log(`Rendered ${total} frames to ${opts.out} in ${elapsed.toFixed(0)} s`);
    } else {
      // The poster the player shows until someone presses play.
      const jpeg = await page.evaluate(
        ({ w, h }) => {
          window.reel.render(window.ctx, Showreel.POSTER_TIME, w, h);
          return document.getElementById("reel").toDataURL("image/jpeg", 0.9);
        },
        { w: WIDTH, h: HEIGHT },
      );
      writeFileSync(posterPartial, Buffer.from(jpeg.slice(jpeg.indexOf(",") + 1), "base64"));
      if (pageError) throw pageError;
      for (const encoder of encoders) renameSync(encoder.partial, encoder.out);
      renameSync(posterPartial, poster);
      const elapsed = (performance.now() - started) / 1000;
      console.log(
        `Rendered ${encoders.map((encoder) => encoder.out).join(", ")} and ${poster} in ${elapsed.toFixed(0)} s`,
      );
    }
  }
} finally {
  // An encoder still running here was cut off by an error; its output is
  // discarded, so stop it without letting it finish the file.
  for (const encoder of encoders) encoder.ffmpeg?.kill("SIGKILL");
  await browser.close();
  rmSync(work, { recursive: true, force: true });
  for (const encoder of encoders) rmSync(encoder.partial, { force: true });
  if (!draft) rmSync(posterPartial, { force: true });
}
