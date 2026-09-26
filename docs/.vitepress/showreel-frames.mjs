// Renders chosen frames of the showreel to PNG, for reviewing a scene while
// it is built: one section, bundled with only its own scene module (so a
// syntax error in another scene cannot break it), or the whole reel. Frames
// are drawn exactly as the video renderer draws them (showreel-video.mjs):
// the same compositor, both bundled fonts loaded with FontFace, in
// Playwright's headless Chromium, and optionally laid out on a labelled
// contact sheet.
//
//   node .vitepress/showreel-frames.mjs (--section <id> | --all)
//        [--beats 0,1.5,4 | --times 12.5,40.25 | --every <beats>]
//        [--facts both|one|none|live] [--raw] [--scale 0.5]
//        [--sheet | --sheet-only] [--cols 8] [--thumb 480] --out <dir>
//
//   --section <id>  one section (open, config, commit, … end); --beats are
//                   local to it, and --every defaults to 1 beat, plus the
//                   section's last frame (len − 1/120 s) to check its hold
//   --all           every scene; --beats are the reel's own, and --every
//                   defaults to 4 beats (a bar), plus the reel's last frame
//   --beats         explicit beats; --times explicit global seconds
//   --every <b>     every b beats from the start; combines with the others
//   --facts         the benchmark facts: both races (the frozen run in
//                   test/results-36078397814.json, the default), one (only
//                   Fix every file, test/published.ts oneClaim), none
//                   (facts null), or live (benchmark/results.json)
//   --raw           no vignette, captions or grain (compare scene pixels)
//   --scale <k>     output size as a fraction of 1920×1080 (default 1)
//   --sheet         also write a labelled contact sheet; --sheet-only skips
//                   the frames; --cols and --thumb (px wide) size its cells
//   --out <dir>     where to write; created if missing
//
// Frames are named by global time, `t040.250_catch_b8.50.png`, and every
// written path is printed. Needs Playwright's Chromium headless shell
// (`aube exec playwright-core install chromium-headless-shell` in docs/) or
// a Chromium-based browser at CHROME_PATH. Exits non-zero if a frame fails;
// the others are still written.

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { availableParallelism, tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { build } from "esbuild";
import { chromium } from "playwright-core";

const here = dirname(fileURLToPath(import.meta.url));
const showreel = join(here, "theme/showreel");
const W = 1920;
const H = 1080;

// The temporary directory the node bundle is loaded from, while it exists;
// fail() removes it, since process.exit skips finally blocks.
let work = null;

function fail(message) {
  if (work) rmSync(work, { recursive: true, force: true });
  console.error(`showreel-frames: ${message}`);
  process.exit(2);
}

function parseArgs(argv) {
  const o = { facts: "both", scale: 1, raw: false, sheet: false, sheetOnly: false };
  const value = (i, flag) => {
    const v = argv[i + 1];
    if (v === undefined || v.startsWith("--")) fail(`${flag} needs a value`);
    return v;
  };
  const numbers = (v, flag) =>
    v.split(",").map((s) => {
      const n = Number(s.trim());
      if (s.trim() === "" || !Number.isFinite(n)) fail(`${flag}: "${s}" is not a number`);
      return n;
    });
  const count = (v, flag) => {
    const n = Number(v);
    if (!/^\s*\d+\s*$/.test(v) || !(n > 0)) fail(`${flag} must be a positive whole number, not "${v}"`);
    return n;
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    switch (a) {
      case "--section": o.section = value(i++, a); break;
      case "--all": o.all = true; break;
      case "--beats": o.beats = numbers(value(i++, a), a); break;
      case "--times": o.times = numbers(value(i++, a), a); break;
      case "--every": [o.every] = numbers(value(i++, a), a); if (!(o.every > 0)) fail("--every must be positive"); break;
      case "--facts": o.facts = value(i++, a); break;
      case "--raw": o.raw = true; break;
      case "--scale": [o.scale] = numbers(value(i++, a), a); if (!(o.scale > 0 && o.scale <= 2)) fail("--scale must be in (0, 2]"); break;
      case "--sheet": o.sheet = true; break;
      case "--sheet-only": o.sheet = o.sheetOnly = true; break;
      case "--cols": o.cols = count(value(i++, a), a); break;
      case "--thumb": o.thumb = count(value(i++, a), a); break;
      case "--out": o.out = value(i++, a); break;
      case "-h":
      case "--help": {
        const head = readFileSync(fileURLToPath(import.meta.url), "utf8").split("\n\nimport")[0];
        console.log(head.replace(/^\/\/ ?/gm, ""));
        process.exit(0);
      }
      default: fail(`unknown argument "${a}" (see --help)`);
    }
  }
  if (!o.section === !o.all) fail("pass exactly one of --section <id> or --all");
  if (!o.out) fail("--out <dir> is required");
  if (!["both", "one", "none", "live"].includes(o.facts)) fail(`--facts must be both, one, none or live, not "${o.facts}"`);
  return o;
}

const opts = parseArgs(process.argv.slice(2));
const out = resolve(opts.out);
const cwd = process.cwd();
const started = performance.now();

// The timeline and the facts variants, run in node from the same sources the
// tests use. test/repo.ts finds the checkout from the working directory.
process.chdir(here);
work = mkdtempSync(join(tmpdir(), "showreel-frames-"));
let node;
try {
  const bundled = await build({
    stdin: {
      contents: `export { BEAT, DURATION, SECTIONS, sec } from "./theme/showreel/timeline.ts";
export { factsFor } from "./theme/showreel/test/published.ts";`,
      resolveDir: here,
      loader: "ts",
    },
    absWorkingDir: here,
    bundle: true,
    platform: "node",
    format: "esm",
    target: "node22",
    write: false,
    logLevel: "silent",
  });
  const file = join(work, "node.mjs");
  writeFileSync(file, bundled.outputFiles[0].text);
  node = await import(pathToFileURL(file).href);
} catch (err) {
  fail(`could not load the timeline and facts:\n${err.message}`);
} finally {
  // Loaded, so the file is no longer needed.
  rmSync(work, { recursive: true, force: true });
  work = null;
}
const { BEAT, DURATION, SECTIONS, sec, factsFor } = node;

const ids = SECTIONS.map((s) => s.id);
if (opts.section && !ids.includes(opts.section)) fail(`no section "${opts.section}"; sections are ${ids.join(", ")}`);
const span = opts.section ? sec(opts.section) : { start: 0, end: DURATION, len: DURATION };
const sectionAt = (t) => SECTIONS.map((s) => sec(s.id)).find((s) => t < s.end) ?? sec(ids[ids.length - 1]);

/** The frames to draw, global seconds, in order and without repeats. */
function frameTimes() {
  const ts = [];
  for (const b of opts.beats ?? []) ts.push(span.start + b * BEAT);
  for (const t of opts.times ?? []) ts.push(t);
  const every = opts.every ?? (opts.beats || opts.times ? null : opts.section ? 1 : 4);
  if (every) {
    for (let k = 0; span.start + k * every * BEAT < span.end - 1e-9; k++) ts.push(span.start + k * every * BEAT);
    // The last frame before the bar line: where a scene rests on the handoff it owes.
    ts.push(span.end - 1 / 120);
  }
  const bad = ts.filter((t) => t < span.start - 1e-9 || t >= span.end - 1e-9);
  if (bad.length) {
    const where = opts.section ? `section ${opts.section} (${span.start}–${span.end} s; its end is the next section's frame)` : `the reel (0–${DURATION} s)`;
    fail(`${bad.map((t) => `${+t.toFixed(4)} s`).join(", ")} outside ${where}`);
  }
  return [...new Set(ts.map((t) => Math.round(t * 1e6) / 1e6))].sort((a, b) => a - b);
}

const times = frameTimes();
if (!times.length) fail("no frames to draw");
const labelOf = (t) => {
  const s = sectionAt(t);
  const b = (t - s.start) / BEAT;
  return { name: `t${t.toFixed(3).padStart(7, "0")}_${s.id}_b${b.toFixed(2)}`, text: `${t.toFixed(3)} s · ${s.id} b${+b.toFixed(3)}` };
};

let facts;
try {
  facts = factsFor(opts.facts);
} catch (err) {
  fail(`could not build the "${opts.facts}" facts: ${err.message}`);
}

let bundle;
try {
  const entry = opts.section
    ? `export { scene } from "./theme/showreel/scenes/${opts.section}.ts";`
    : `export { scenes } from "./theme/showreel/scenes/index.ts";`;
  bundle = await build({
    stdin: {
      contents: `export { composeReel } from "./theme/showreel/compose.ts";
export { resetTypeCache } from "./theme/showreel/type.ts";
${entry}`,
      resolveDir: here,
      loader: "ts",
    },
    absWorkingDir: here,
    bundle: true,
    format: "iife",
    globalName: "Showreel",
    target: "es2022",
    write: false,
    logLevel: "silent",
  });
} catch (err) {
  const errors = (err.errors ?? []).map((e) => `  ${e.location ? `${relative(cwd, resolve(here, e.location.file))}:${e.location.line}:${e.location.column}: ` : ""}${e.text}`);
  fail(`the ${opts.section ? `${opts.section} scene` : "reel"} does not bundle:\n${errors.join("\n") || err.message}`);
}

const pw = Math.round(W * opts.scale);
const ph = Math.round(H * opts.scale);
const thumbW = Math.round(opts.thumb ?? Math.min(pw, 480));
const thumbH = Math.round((thumbW * H) / W);
const fonts = Object.fromEntries(
  ["SpaceGrotesk.ttf", "LiberationMono-Regular.ttf", "LiberationMono-Bold.ttf"].map((f) => [f, readFileSync(join(here, "fonts", f)).toString("base64")]),
);

mkdirSync(out, { recursive: true });
const written = [];
const failures = [];
let broken = null;
const browser = await chromium.launch({ executablePath: process.env.CHROME_PATH || undefined });
try {
  const PAGES = Math.max(1, Math.min(4, availableParallelism() >> 1, times.length));
  const pages = await Promise.all(
    Array.from({ length: PAGES }, async () => {
      const page = await browser.newPage();
      const errors = [];
      page.on("pageerror", (err) => errors.push(err));
      await page.setContent(`<canvas id="reel" width="${pw}" height="${ph}"></canvas>`);
      await page.addScriptTag({ content: bundle.outputFiles[0].text });
      // A scene that throws as it loads leaves no Showreel global: report its own error.
      await page.evaluate(
        async ({ fonts, facts, raw, section, thumbW, thumbH }) => {
          const bytes = (b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
          const faces = [
            new FontFace("Space Grotesk", bytes(fonts["SpaceGrotesk.ttf"]), { weight: "300 700" }),
            new FontFace("Liberation Mono", bytes(fonts["LiberationMono-Regular.ttf"]), { weight: "400" }),
            new FontFace("Liberation Mono", bytes(fonts["LiberationMono-Bold.ttf"]), { weight: "700" }),
          ];
          for (const face of faces) document.fonts.add(await face.load());
          // layout() caches measurements; none may survive from a fallback font.
          Showreel.resetTypeCache();
          window.reel = Showreel.composeReel(section ? [Showreel.scene] : Showreel.scenes, facts, { raw });
          window.canvas = document.getElementById("reel");
          window.ctx = window.canvas.getContext("2d", { alpha: false });
          window.thumb = document.createElement("canvas");
          window.thumb.width = thumbW;
          window.thumb.height = thumbH;
        },
        { fonts, facts, raw: opts.raw, section: opts.section ?? null, thumbW, thumbH },
      ).catch((err) => {
        throw errors[0] ?? err;
      });
      if (errors.length) throw errors[0];
      return { page, errors };
    }),
  );

  const results = new Array(times.length);
  await Promise.all(
    pages.map(async ({ page, errors }, p) => {
      for (let i = p; i < times.length; i += PAGES) {
        const t = times[i];
        const label = labelOf(t);
        try {
          const shot = await page.evaluate(
            ({ t, pw, ph, full, sheet }) => {
              window.reel.render(window.ctx, t, pw, ph);
              let thumb = null;
              if (sheet) {
                const g = window.thumb.getContext("2d");
                g.imageSmoothingQuality = "high";
                g.drawImage(window.canvas, 0, 0, window.thumb.width, window.thumb.height);
                thumb = window.thumb.toDataURL("image/jpeg", 0.92);
              }
              return { png: full ? window.canvas.toDataURL("image/png") : null, thumb };
            },
            { t, pw, ph, full: !opts.sheetOnly, sheet: opts.sheet },
          );
          if (errors.length) throw errors.splice(0)[0];
          if (shot.png) {
            const file = join(out, `${label.name}.png`);
            writeFileSync(file, Buffer.from(shot.png.slice(shot.png.indexOf(",") + 1), "base64"));
            written.push(file);
          }
          results[i] = { ...label, thumb: shot.thumb };
        } catch (err) {
          failures.push(`${label.text}: ${err.stack ?? err.message ?? err}`);
          results[i] = { ...label, thumb: null, failed: true };
        }
      }
    }),
  );

  if (opts.sheet) {
    const cols = Math.max(1, Math.min(Math.round(opts.cols ?? 8), results.length));
    const title = `hk showreel · ${opts.section ?? "all"} · facts ${opts.facts}${opts.raw ? " · raw" : ""} · ${results.length} frames`;
    const png = await pages[0].page.evaluate(
      async ({ cells, cols, thumbW, thumbH, title }) => {
        const PAD = 8;
        const LABEL = 30;
        const HEAD = 44;
        const rows = Math.ceil(cells.length / cols);
        const sheet = document.createElement("canvas");
        sheet.width = PAD + cols * (thumbW + PAD);
        sheet.height = HEAD + rows * (thumbH + LABEL + PAD) + PAD;
        const g = sheet.getContext("2d");
        g.fillStyle = "#1a1f24";
        g.fillRect(0, 0, sheet.width, sheet.height);
        g.fillStyle = "#e8edf0";
        g.font = '600 24px "Space Grotesk", sans-serif';
        g.fillText(title, PAD + 4, 30);
        for (const [i, c] of cells.entries()) {
          const x = PAD + (i % cols) * (thumbW + PAD);
          const y = HEAD + Math.floor(i / cols) * (thumbH + LABEL + PAD);
          if (c.thumb) {
            const img = new Image();
            img.src = c.thumb;
            await img.decode();
            g.drawImage(img, x, y, thumbW, thumbH);
          } else {
            g.fillStyle = "#5a1f24";
            g.fillRect(x, y, thumbW, thumbH);
          }
          g.fillStyle = c.failed ? "#e78284" : "#adbdc9";
          g.font = '400 18px "Liberation Mono", monospace';
          g.fillText(c.failed ? `${c.text} FAILED` : c.text, x + 2, y + thumbH + 21, thumbW - 4);
        }
        return sheet.toDataURL("image/png");
      },
      { cells: results, cols, thumbW, thumbH, title },
    );
    const file = join(out, `sheet-${opts.section ?? "all"}-${opts.facts}${opts.raw ? "-raw" : ""}.png`);
    writeFileSync(file, Buffer.from(png.slice(png.indexOf(",") + 1), "base64"));
    written.push(file);
  }
} catch (err) {
  // Only setting a page up can land here; a frame's own failure is caught per frame.
  broken = err;
} finally {
  await browser.close();
}
if (broken) {
  console.error(`showreel-frames: the ${opts.section ? `${opts.section} scene` : "reel"} failed to load:\n${broken.stack ?? broken.message ?? broken}`);
  process.exit(1);
}

for (const file of written) console.log(file);
const seconds = ((performance.now() - started) / 1000).toFixed(1);
console.error(`${times.length} frames of ${opts.section ?? "the reel"} (facts ${opts.facts}) in ${seconds} s`);
if (failures.length) {
  console.error(`\n${failures.length} frame(s) failed:\n${failures.join("\n\n")}`);
  process.exit(1);
}
