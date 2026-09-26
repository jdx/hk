// K2 against the real runs it redraws (test/captures, README there): every
// terminal line in screens.ts is a line hk printed, header() prints hk's
// header character for character, the styling rules give every character
// the colour hk's own ANSI gave it, every character is one Liberation Mono
// has or one term.ts draws itself, and the panes show the rows the
// storyboard's beats name.

import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { TERM } from "../bible";
import { blocked, checkAll, commit, final, finding, LOG, PROMPT_COMMIT, type Screen, VERSION } from "../kit/screens";
import {
  advance,
  firstLine,
  header,
  INSET,
  isVectorGlyph,
  mini,
  PANE_FULL,
  type Pane,
  SPINNER,
  spinnerFrame,
  STRIP,
  styleLine,
  termLayout,
} from "../kit/term";
import { REPO, SHOWREEL } from "./repo";

const CAPTURES = join(SHOWREEL, "test/captures");
const read = (file: string): string => readFileSync(join(CAPTURES, file), "utf8");
const FILES = readdirSync(CAPTURES).sort();
const TEXTS = FILES.filter((f) => f.endsWith(".frames.txt") || f.endsWith(".screen.txt"));

/** A capture's screens by frame number, or "final" for a .screen.txt. */
function screens(file: string): Map<string, string[]> {
  const out = new Map<string, string[]>();
  const parts = read(file).split(/^----- (?:frame )?(\d+|final) -----\n/m);
  for (let i = 1; i < parts.length; i += 2) out.set(parts[i], parts[i + 1].replace(/\n+$/, "").split("\n"));
  return out;
}

/** Every line of every frame and final screen. */
const CAPTURED = new Set(TEXTS.flatMap((f) => [...screens(f).values()].flat()));

/** Every screen in screens.ts, named. */
const SCREENS: [string, Screen][] = [
  ...Object.entries(commit).map(([f, s]): [string, Screen] => [`commit[${f}]`, s]),
  ...blocked.map((s, f): [string, Screen] => [`blocked[${f}]`, s]),
  ...checkAll.map((s, f): [string, Screen] => [`checkAll[${f}]`, s]),
  ...Object.entries(final).map(([k, s]): [string, Screen] => [`final.${k}`, s]),
];

test("every screen in screens.ts is its capture's frame of the same number, line for line", () => {
  const c = screens("commit.frames.txt");
  for (const [f, s] of Object.entries(commit)) assert.deepEqual(s, c.get(f), `commit[${f}]`);
  const b = screens("blocked.frames.txt");
  blocked.forEach((s, f) => assert.deepEqual(s, b.get(String(f)), `blocked[${f}]`));
  const k = screens("check-all.frames.txt");
  checkAll.forEach((s, f) => assert.deepEqual(s, k.get(String(f)), `checkAll[${f}]`));
  assert.deepEqual(final.commit, screens("commit.screen.txt").get("final"));
  assert.deepEqual(final.fix, screens("fix.screen.txt").get("final"));
  assert.deepEqual(final.checkAll, screens("check-all.screen.txt").get("final"));
  // The storyboard's frames: the ones its beats name are the ones it describes.
  assert.equal(commit[1][1], "✔ files - Fetching staged files (4 files)");
  assert.equal(commit[2][1], "✔ stash – Stashed unstaged changes (1 file)");
  assert.deepEqual(commit[22], final.commit, "the commit's last frame is its final screen");
  assert.equal(blocked[11][3], "✗ shellcheck");
  assert.equal(blocked[12][3], "✗ shellcheck  – ERROR");
  assert.equal(blocked[13].at(-1), "✔ stash – Restoring unstaged changes (manual)");
  assert.equal(checkAll[15][0], header(null, "check", 7, 7));
  assert.deepEqual(checkAll[16], checkAll[15]);
});

test("every string in screens.ts occurs verbatim in the captures", () => {
  for (const [name, s] of SCREENS) for (const line of s) assert.ok(CAPTURED.has(line), `${name}: ${JSON.stringify(line)}`);
  const stderr = screens("blocked.screen.txt").get("final")!;
  assert.ok(stderr.some((l) => l.includes(finding.file)));
  assert.ok(stderr.includes(finding.source));
  assert.ok(stderr.some((l) => l.startsWith(finding.warning)));
  assert.deepEqual(LOG, read("log.txt").trimEnd().split("\n"));
  assert.deepEqual(read("log-after-blocked.txt"), read("log.txt"), "the blocked commit made no commit");
  // The prompt is typed, not printed: its message is the commit's.
  const message = /-m "([^"]+)"$/.exec(PROMPT_COMMIT)?.[1];
  assert.ok(message && PROMPT_COMMIT.startsWith("$ git commit -m "));
  assert.ok(CAPTURED.has(`[main ${LOG[0].split(" ")[0]}] ${message}`));
  assert.equal(LOG[0], `ada2ca4 ${message}`);
});

test("the captures hold no machine paths", () => {
  for (const f of FILES) assert.doesNotMatch(read(f), /\/tmp|\/home|claude|scratchpad/i, f);
});

// hk's header.

const HEADER_LINE = /^hk (\S+) by @jdx(?: – (pre-commit))? – (fix|check) {2}\[[=> ]*\] (\d+)\/(\d+)$/;

test("header() reproduces every header line in the captures", () => {
  let n = 0;
  for (const line of CAPTURED) {
    const m = HEADER_LINE.exec(line);
    if (!m) continue;
    n++;
    assert.equal(m[1], VERSION, "the captures were made with VERSION");
    assert.equal(header(m[2] ? "pre-commit" : null, m[3] as "fix" | "check", Number(m[4]), Number(m[5])), line);
  }
  // The pre-commit bar at 0–7/7 and 0–4/4, check's at 0–7/7, fix's at 0–7/7.
  assert.ok(n >= 25, `${n} distinct header lines`);
  for (const line of CAPTURED) if (line.startsWith("hk ") && line.includes(" by @jdx")) assert.match(line, HEADER_LINE);
});

test("header() fills the width as clx's bar does", () => {
  const cells = (s: string) => s.slice(s.indexOf("[") + 1, s.lastIndexOf("]")).length;
  // Checked by hand against pre-commit (37 cells), check (48) and fix (50).
  assert.equal(cells(header("pre-commit", "fix", 0, 7)), 37);
  assert.equal(cells(header(null, "check", 0, 7)), 48);
  assert.equal(cells(header(null, "fix", 0, 7)), 50);
  assert.equal(header("pre-commit", "fix", 0, 7), `hk ${VERSION} by @jdx – pre-commit – fix  [${" ".repeat(37)}] 0/7`);
  assert.equal(header(null, "check", 7, 7), `hk ${VERSION} by @jdx – check  [${"=".repeat(48)}] 7/7`);
  for (const [cur, total] of [[0, 7], [3, 7], [7, 7], [1, 4], [2, 4], [10, 10], [9, 12]]) {
    for (const cols of [72, 80, 100]) assert.equal(header("pre-commit", "fix", cur, total, cols).length, cols, `${cur}/${total} at ${cols}`);
  }
  // A start too small to round to one cell draws none; a half rounds up.
  assert.ok(!header(null, "fix", 1, 200).includes(">"));
  assert.ok(header("pre-commit", "fix", 2, 4).includes(`[${"=".repeat(18)}>`), "37 × 2/4 = 18.5 rounds to 19 cells");
  assert.equal(header(null, "fix", 0, 0).includes("="), false);
});

// Colours.

/** The colour an SGR state shows in, as identity §2.2 maps hk's ANSI onto Catppuccin Frappé. */
function sgrColor(fg: string | null, dim: boolean): string {
  if (fg) return ({ "35": TERM.magenta, "31": TERM.red, "33": TERM.yellow, "34": TERM.blue, "38;5;10": TERM.green } as Record<string, string>)[fg];
  return dim ? TERM.dim : TERM.text;
}

/** styles.txt: every distinct line hk printed, its SGR escapes written as `\e[…m`. */
function styled(): { text: string; chars: { color: string; bold: boolean }[] }[] {
  return read("styles.txt")
    .trimEnd()
    .split("\n")
    .map((line) => {
      let fg: string | null = null;
      let bold = false;
      let dim = false;
      let text = "";
      const chars: { color: string; bold: boolean }[] = [];
      for (const part of line.split(/(\\e\[[\d;]*m)/)) {
        const m = /^\\e\[([\d;]*)m$/.exec(part);
        if (!m) {
          for (const ch of part) {
            text += ch;
            chars.push({ color: sgrColor(fg, dim), bold });
          }
          continue;
        }
        const codes = m[1].split(";");
        for (let i = 0; i < codes.length; i++) {
          const c = codes[i];
          if (c === "0" || c === "") [fg, bold, dim] = [null, false, false];
          else if (c === "1") bold = true;
          else if (c === "2") dim = true;
          else if (c === "38") {
            fg = codes.slice(i, i + 3).join(";");
            i += 2;
          } else fg = c;
          assert.ok(fg === null || sgrColor(fg, false), `an SGR code the map covers: ${m[1]}`);
        }
      }
      return { text, chars };
    });
}

/** styleLine as one colour and weight per character. */
const perChar = (text: string) => styleLine(text).flatMap((r) => Array.from(r.text, () => ({ color: r.color, bold: r.bold })));

test("styleLine gives every character the colour hk's ANSI gave it", () => {
  const lines = styled();
  assert.ok(lines.length > 60, `${lines.length} styled lines`);
  for (const { text, chars } of lines) {
    const got = perChar(text);
    assert.equal(got.length, chars.length, text);
    Array.from(text).forEach((ch, i) => {
      // A space's colour never shows.
      if (ch === " ") return;
      assert.deepEqual(got[i], chars[i], `${JSON.stringify(text)} column ${i} (${ch})`);
    });
  }
});

test("every line the reel shows is among the styled ones", () => {
  const plain = new Set(styled().map((l) => l.text));
  for (const [name, s] of SCREENS) for (const line of s) assert.ok(plain.has(line), `${name}: ${JSON.stringify(line)}`);
});

test("styleLine classifies the lines the captures do not print", () => {
  const colors = (text: string) => styleLine(text).map((r) => [r.text, r.color, r.bold]);
  assert.deepEqual(colors(PROMPT_COMMIT), [
    ["$ ", TERM.prompt, false],
    ['git commit -m "feat: hoist the sails"', TERM.text, false],
  ]);
  assert.deepEqual(colors("⚠ prettier  – aborted"), [
    ["⚠", TERM.yellow, false],
    [" prettier  – ", TERM.text, false],
    ["aborted", TERM.yellow, false],
  ]);
  assert.deepEqual(colors("hk ERROR To fix, run: shfmt -w --apply-ignore scripts/deploy.sh"), [
    ["hk", TERM.red, false],
    [" ", TERM.text, false],
    ["ERROR", TERM.red, false],
    [" To fix, run: ", TERM.text, false],
    ["shfmt -w --apply-ignore scripts/deploy.sh", TERM.dim, false],
  ]);
  assert.deepEqual(colors("shellcheck stderr:"), [["shellcheck stderr:", TERM.text, true]]);
  // Tool and git output, and a line only mentioning a glyph, stay plain.
  for (const text of ["[main ada2ca4] feat: hoist the sails", " README.md 12ms", "unused=1", "hk.pkl ✔ later", "✔done"]) {
    assert.deepEqual(colors(text), [[text, TERM.text, false]], text);
  }
});

// Glyphs.

/** The code points a TrueType font maps to a glyph (its Unicode cmap subtables, formats 4 and 12). */
function covered(file: string): Set<number> {
  const b = readFileSync(file);
  const tables = b.readUInt16BE(4);
  let cmap = -1;
  for (let i = 0; i < tables; i++) if (b.toString("latin1", 12 + 16 * i, 16 + 16 * i) === "cmap") cmap = b.readUInt32BE(20 + 16 * i);
  assert.ok(cmap > 0, `${file} has a cmap`);
  const out = new Set<number>();
  for (let i = 0; i < b.readUInt16BE(cmap + 2); i++) {
    const rec = cmap + 4 + 8 * i;
    const [platform, encoding] = [b.readUInt16BE(rec), b.readUInt16BE(rec + 2)];
    const sub = cmap + b.readUInt32BE(rec + 4);
    const format = b.readUInt16BE(sub);
    if (platform !== 3 && platform !== 0) continue;
    if (format === 4 && (platform === 0 || encoding === 1)) {
      const segs = b.readUInt16BE(sub + 6) / 2;
      const ends = sub + 14;
      const starts = ends + 2 * segs + 2;
      const deltas = starts + 2 * segs;
      const offsets = deltas + 2 * segs;
      for (let s = 0; s < segs; s++) {
        const [start, end] = [b.readUInt16BE(starts + 2 * s), b.readUInt16BE(ends + 2 * s)];
        const [delta, offset] = [b.readInt16BE(deltas + 2 * s), b.readUInt16BE(offsets + 2 * s)];
        for (let c = start; c <= end && c !== 0xffff; c++) {
          let g = offset ? b.readUInt16BE(offsets + 2 * s + offset + 2 * (c - start)) : c;
          if (offset && !g) continue;
          g = (g + delta) & 0xffff;
          if (g) out.add(c);
        }
      }
    } else if (format === 12) {
      for (let g = 0; g < b.readUInt32BE(sub + 12); g++) {
        const grp = sub + 16 + 12 * g;
        for (let c = b.readUInt32BE(grp); c <= b.readUInt32BE(grp + 4); c++) out.add(c);
      }
    }
  }
  return out;
}

test("every character is Liberation Mono's, or one term.ts draws in its cell", () => {
  const fonts = join(REPO, "docs/.vitepress/fonts");
  const regular = covered(join(fonts, "LiberationMono-Regular.ttf"));
  const bold = covered(join(fonts, "LiberationMono-Bold.ttf"));
  assert.ok(regular.has(0x41) && regular.has(0x2013) && regular.has(0x2026), "the cmap reads: A – …");
  // Why term.ts draws them: the font has none of them.
  for (const ch of "✔✗❯⚠" + SPINNER) {
    assert.ok(isVectorGlyph(ch), ch);
    assert.ok(!regular.has(ch.codePointAt(0)!) && !bold.has(ch.codePointAt(0)!), `${ch} is not in Liberation Mono`);
  }
  const lines = [PROMPT_COMMIT, ...SCREENS.flatMap(([, s]) => s)];
  for (const line of lines) {
    for (const r of styleLine(line)) {
      const face = r.bold ? bold : regular;
      for (const ch of r.text) assert.ok(isVectorGlyph(ch) || face.has(ch.codePointAt(0)!), `${JSON.stringify(ch)} in ${JSON.stringify(line)}`);
    }
  }
});

test("the spinner is clx's mini_dot, a frame per 200 ms of global time", () => {
  assert.equal(SPINNER, "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
  assert.equal(spinnerFrame(0), "⠋");
  assert.equal(spinnerFrame(0.2), "⠙");
  assert.equal(spinnerFrame(0.6), "⠸", "0.6 / 0.2 is frame 3, not 2.999…");
  assert.equal(spinnerFrame(1.999), "⠏");
  assert.equal(spinnerFrame(2), "⠋");
  assert.equal(spinnerFrame(40.25), spinnerFrame(0.25), "a pure function of global t");
  for (const line of CAPTURED) for (const ch of line) if (ch >= "⠀" && ch <= "⣿") assert.ok(SPINNER.includes(ch), ch);
});

// Panes.

const baselines = (p: Pane) => Array.from({ length: p.rows }, (_, r) => termLayout(p, p.rows, 0).baseline(r));

test("the presets put rows and columns where the storyboard's table does", () => {
  assert.deepEqual(baselines(PANE_FULL), [234, 278, 322, 366, 410, 454, 498, 542, 586, 630, 674]);
  assert.deepEqual(baselines(STRIP), [190, 234]);
  assert.deepEqual(baselines(INSET), [147, 183, 219, 255, 291, 327, 363, 399]);
  assert.deepEqual(baselines(mini(160)), [417, 449, 481, 513, 545, 577, 609, 641, 673]);
  assert.deepEqual([PANE_FULL.x0, STRIP.x0, INSET.x0, mini(700).x0], [192, 192, 140, 718]);
  assert.equal(advance(32), 19.2);
  for (const p of [PANE_FULL, STRIP]) {
    // 80 columns fit inside the window, so nothing needs a fade.
    assert.equal(p.fade, 0);
    assert.ok(termLayout(p, 1).col(80) <= p.x + p.w - 16);
  }
  for (const p of [INSET, mini(160)]) assert.ok(termLayout(p, 1).col(80) > p.x + p.w, "80 columns run past the window, under the fade");
  assert.deepEqual([INSET.fade, mini(160).fade], [60, 48]);
  for (const p of [PANE_FULL, STRIP, INSET, mini(1240)]) {
    const L = termLayout(p, p.rows, 0);
    // Every row's ink, cap height to descender, is inside the body.
    assert.ok(L.baseline(0) - 0.8 * p.size >= L.body.y, "row 0 clears the chrome");
    assert.ok(L.baseline(p.rows - 1) + 0.25 * p.size <= p.y + p.h, "the last row clears the bottom edge");
    assert.equal(L.cell(2, 5).x, p.x0 + 5 * advance(p.size));
    assert.equal(L.cell(2, 5).h, p.lineH);
  }
  assert.deepEqual([PANE_FULL.chrome, STRIP.chrome, INSET.chrome, mini(160).chrome], [true, true, false, false]);
  assert.deepEqual([PANE_FULL.window, INSET.window, mini(160).window], [true, true, false]);
});

test("a pane shows a screen's bottom rows; MINI and a top-anchored pane show the top", () => {
  const shown = (p: Pane, s: Screen) => s.slice(firstLine(p, s.length), firstLine(p, s.length) + p.rows);
  // commit|stash: the prompt, the header at 0/7 and the files row, from row 0.
  assert.deepEqual(shown(PANE_FULL, [PROMPT_COMMIT, ...commit[1]]), [PROMPT_COMMIT, header("pre-commit", "fix", 0, 7), "✔ files - Fetching staged files (4 files)"]);
  // stash: the strip keeps the header and the files row as the prompt scrolls away.
  assert.deepEqual(shown(STRIP, [PROMPT_COMMIT, ...commit[1]]), commit[1]);
  assert.deepEqual(shown(STRIP, commit[2]), [header("pre-commit", "fix", 0, 7), "✔ stash – Stashed unstaged changes (1 file)"]);
  // restore: the strip's two rows of frames 19 and 22.
  assert.deepEqual(shown(STRIP, commit[19]), ["✔ newlines", "✔ stash – Restoring unstaged changes (manual)"]);
  assert.deepEqual(shown(STRIP, commit[22]), ["[main ada2ca4] feat: hoist the sails", " 3 files changed, 8 insertions(+), 1 deletion(-)"]);
  // catch: every blocked frame fits the inset whole.
  for (const s of blocked) assert.deepEqual(shown(INSET, s), s);
  // everywhere: the top 9 rows of each final screen, header first.
  for (const s of Object.values(final)) assert.deepEqual(shown(mini(160), s), s.slice(0, 9));
  // race F0: the top 11 rows of each check-all frame, the header always on top.
  for (const s of checkAll) assert.equal(shown({ ...PANE_FULL, anchor: "top" }, s)[0], s[0]);
  assert.equal(firstLine(PANE_FULL, final.commit.length), 1, "the full pane scrolls the commit's header away");
});
