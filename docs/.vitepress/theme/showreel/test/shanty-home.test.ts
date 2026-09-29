// Sea shanty mode's landing page: the colours shanty-home.css mixes from the
// mode's palette, and the text that sits on them. The palette itself is
// checked by shanty-mode.test.ts; this reads it the same way, so a change to
// either file is checked against the other.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { REPO } from "./repo";

/** Reads the custom properties declared in the first block for `selector`. */
function tokens(css: string, selector: string): Record<string, string> {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\:]/g, "\\$&");
  const block = new RegExp(`(?:^|\\n)${escaped}\\s*\\{([^}]*)\\}`).exec(css.replace(/\/\*[\s\S]*?\*\//g, ""));
  if (!block) return {};
  const out: Record<string, string> = {};
  for (const declaration of block[1].split(";")) {
    const at = declaration.indexOf(":");
    if (at > 0 && declaration.slice(0, at).trim().startsWith("--")) {
      out[declaration.slice(0, at).trim()] = declaration.slice(at + 1).trim().replace(/\s+/g, " ");
    }
  }
  return out;
}

/** Red, green and blue from 0 to 255, and alpha from 0 to 1. */
type Rgba = [number, number, number, number];

/**
 * A token's colour, as the browser works it out: a hex colour, `rgb(r g b /
 * a%)`, `transparent`, another token, or `color-mix(in srgb, a p%, b)`, which
 * mixes with premultiplied alpha, so a colour mixed with `transparent` keeps
 * its hue and takes the share as its alpha.
 */
function resolve(value: string, t: Record<string, string>, depth = 0): Rgba {
  assert.ok(depth < 20, `${value} refers to itself`);
  const v = value.trim();
  const ref = /^var\((--[\w-]+)\)$/.exec(v);
  if (ref) {
    assert.ok(t[ref[1]], `${ref[1]} is not set`);
    return resolve(t[ref[1]], t, depth + 1);
  }
  if (v === "transparent") return [0, 0, 0, 0];
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(v);
  if (hex) {
    const full = hex[1].length === 3 ? [...hex[1]].map((c) => c + c).join("") : hex[1];
    const [r, g, b] = [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16));
    return [r, g, b, 1];
  }
  const rgb = /^rgb\((\d+) (\d+) (\d+)(?: \/ (\d+)%)?\)$/.exec(v);
  if (rgb) return [Number(rgb[1]), Number(rgb[2]), Number(rgb[3]), rgb[4] === undefined ? 1 : Number(rgb[4]) / 100];
  const mix = /^color-mix\(in srgb, (.+?) (\d+)%, (.+)\)$/.exec(v);
  assert.ok(mix, `cannot read the colour ${v}`);
  const a = resolve(mix[1], t, depth + 1);
  const b = resolve(mix[3], t, depth + 1);
  const p = Number(mix[2]) / 100;
  const alpha = a[3] * p + b[3] * (1 - p);
  if (alpha === 0) return [0, 0, 0, 0];
  const channel = (i: number) => (a[i] * a[3] * p + b[i] * b[3] * (1 - p)) / alpha;
  return [channel(0), channel(1), channel(2), alpha];
}

/** A colour laid over an opaque one. */
const over = (top: Rgba, under: Rgba): Rgba => [0, 1, 2].map((i) => top[i] * top[3] + under[i] * (1 - top[3])).concat(1) as Rgba;

const luminance = ([r, g, b]: Rgba) => {
  const [lr, lg, lb] = [r, g, b].map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
};

const contrast = (a: Rgba, b: Rgba) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

const theme = join(REPO, "docs/.vitepress/theme");
const base = readFileSync(join(theme, "style.css"), "utf8");
const sea = readFileSync(join(theme, "shanty-mode.css"), "utf8");
const home = readFileSync(join(theme, "shanty-home.css"), "utf8");

/** Each theme as the landing page resolves it: style.css, the mode's palette, then the landing page's own tokens. */
const PALETTES = {
  light: {
    ...tokens(base, ":root"),
    ...tokens(sea, ":root.shanty-mode"),
    ...tokens(sea, ":root:not(.dark).shanty-mode"),
    ...tokens(home, ":root.shanty-mode"),
    ...tokens(home, ":root:not(.dark).shanty-mode"),
  },
  dark: {
    ...tokens(base, ":root"),
    ...tokens(base, ".dark"),
    ...tokens(sea, ":root.shanty-mode"),
    ...tokens(sea, ":root.dark.shanty-mode"),
    ...tokens(home, ":root.shanty-mode"),
    ...tokens(home, ":root.dark.shanty-mode"),
  },
};

test("the landing page mixes its colours from the mode's palette", () => {
  const shared = tokens(home, ":root.shanty-mode");
  for (const name of ["--hk-home-check-bg", "--hk-home-fix-bg", "--hk-home-rule", "--hk-home-rope", "--hk-home-rope-2", "--hk-home-sea"]) {
    assert.match(shared[name] ?? "", /^color-mix\(/, `${name} is mixed, so it follows the palette`);
  }
  for (const selector of [":root:not(.dark).shanty-mode", ":root.dark.shanty-mode"]) {
    assert.ok(tokens(home, selector)["--hk-home-glow"], `${selector} sets --hk-home-glow`);
  }
});

for (const [name, t] of Object.entries(PALETTES)) {
  const at = (token: string) => resolve(`var(${token})`, t);
  const ratio = (text: string, surface: Rgba) => contrast(at(text), surface);
  const bg = at("--vp-c-bg");

  test(`${name}: the lane chips' words read on their opaque chips`, () => {
    for (const [text, chip] of [
      ["--vp-c-brand-1", "--hk-home-check-bg"],
      ["--hk-code-keyword", "--hk-home-fix-bg"],
    ]) {
      assert.equal(at(chip)[3], 1, `${name}: ${chip} is opaque, so the rope does not show through`);
      const r = ratio(text, at(chip));
      assert.ok(r >= 4.5, `${name}: ${text} on ${chip} is ${r.toFixed(2)}:1`);
    }
  });

  test(`${name}: brass and links read on the cards they sit on`, () => {
    // Brass: kickers on the page, the articles' label, the chorus's answers on the song sheet.
    for (const surface of ["--vp-c-bg", "--vp-c-bg-soft", "--vp-c-bg-elv"]) {
      const r = ratio("--hk-sea-brass", at(surface));
      assert.ok(r >= 4.5, `${name}: brass on ${surface} is ${r.toFixed(2)}:1`);
    }
    // Links and code on the articles, the plaques and the song sheet.
    for (const surface of ["--vp-c-bg-soft", "--vp-c-bg-elv"]) {
      const r = ratio("--vp-c-brand-1", at(surface));
      assert.ok(r >= 4.5, `${name}: brand-1 on ${surface} is ${r.toFixed(2)}:1`);
    }
  });

  test(`${name}: the hero's words read where the chart is foxed or lamplit`, () => {
    // The glow is strongest at one end of its gradient; the words may sit anywhere on it.
    const glow = over(at("--hk-home-glow"), bg);
    for (const text of ["--vp-c-text-1", "--vp-c-text-2", "--vp-c-brand-1", "--hk-sea-brass"]) {
      const r = ratio(text, glow);
      assert.ok(r >= 4.5, `${name}: ${text} on the chart's glow is ${r.toFixed(2)}:1`);
    }
  });

  test(`${name}: the destination's ship shows on its brass disc`, () => {
    // Drawn in the page colour on brass: a graphic, so 3:1.
    const r = contrast(bg, at("--hk-sea-brass"));
    assert.ok(r >= 3, `${name}: the page colour on brass is ${r.toFixed(2)}:1`);
  });
}

test("colour mixing is read as browsers mix", () => {
  const t = { "--a": "#ff0000", "--b": "#0000ff" };
  assert.deepEqual(resolve("color-mix(in srgb, var(--a) 25%, var(--b))", t), [63.75, 0, 191.25, 1]);
  assert.deepEqual(resolve("color-mix(in srgb, var(--a) 40%, transparent)", t), [255, 0, 0, 0.4]);
  assert.deepEqual(over([255, 0, 0, 0.4], [0, 0, 255, 1]), [102, 0, 153, 1]);
});
