// Sea shanty mode: its state rules, the script that applies them before the
// first paint, and the palette that shanty-mode.css gives the site.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import vm from "node:vm";
import {
  announcement,
  arrive,
  englishPath,
  fileKey,
  hasVariant,
  isPiratePath,
  pageKey,
  paintShanty,
  paramState,
  piratePath,
  prePaintScript,
  reelKind,
  rememberShanty,
  SHANTY_CLASS,
  SHANTY_KEY,
  stripShantyParam,
  themedAfterNavigation,
  wantsShanty,
} from "../../shanty-mode";
import { REPO } from "./repo";

/** Search strings and what the `?shanty` parameter says in each. */
const PARAMS: [string, boolean | null][] = [
  ["", null],
  ["?", null],
  ["?shanty", true],
  ["?shanty=", true],
  ["?shanty=1", true],
  ["?shanty=on", true],
  ["?shanty=yes", true],
  ["?SHANTY", true],
  ["?shanty=0", false],
  ["?shanty=OFF", false],
  ["?shanty=false", false],
  ["?shanty=No", false],
  ["?a=1&shanty", true],
  ["?a=1&shanty&b=2", true],
  ["?shanty=off&b=2", false],
  ["?shantytown", null],
  ["?x=shanty", null],
  ["?myshanty=1", null],
];

/** A store that answers from a map, or throws where it is told to. */
function store(initial: Record<string, string> = {}, throws = false) {
  const values = new Map(Object.entries(initial));
  const guard = () => {
    if (throws) throw new Error("blocked");
  };
  return {
    values,
    getItem: (key: string) => (guard(), values.get(key) ?? null),
    setItem: (key: string, value: string) => (guard(), void values.set(key, value)),
    removeItem: (key: string) => (guard(), void values.delete(key)),
  };
}

test("the ?shanty parameter is read as a whole parameter", () => {
  for (const [search, expected] of PARAMS) assert.equal(paramState(search), expected, JSON.stringify(search));
});

test("the URL beats the remembered choice, in both directions", () => {
  const on = store({ [SHANTY_KEY]: "1" });
  const off = store();
  assert.equal(wantsShanty("?shanty=0", on), false);
  assert.equal(wantsShanty("?shanty", off), true);
  assert.equal(wantsShanty("", on), true);
  assert.equal(wantsShanty("", off), false);
  assert.equal(wantsShanty("?other=1", on), true);
});

test("only a stored 1 turns the mode on", () => {
  for (const stored of ["0", "true", "on", "", "yes", " 1"]) {
    assert.equal(wantsShanty("", store({ [SHANTY_KEY]: stored })), false, JSON.stringify(stored));
  }
});

test("blocked or missing storage means off, and the URL still works", () => {
  const blocked = store({ [SHANTY_KEY]: "1" }, true);
  assert.equal(wantsShanty("", blocked), false);
  assert.equal(wantsShanty("", null), false);
  assert.equal(wantsShanty("", undefined), false);
  assert.equal(wantsShanty("?shanty", blocked), true);
  assert.equal(wantsShanty("?shanty", null), true);
});

test("the choice is remembered, and forgotten when the mode goes off", () => {
  const s = store();
  rememberShanty(true, s);
  assert.equal(s.values.get(SHANTY_KEY), "1");
  assert.equal(wantsShanty("", s), true);
  rememberShanty(false, s);
  assert.equal(s.values.has(SHANTY_KEY), false);
  assert.equal(wantsShanty("", s), false);
});

test("a blocked store does not break turning the mode on or off", () => {
  const blocked = store({}, true);
  assert.doesNotThrow(() => rememberShanty(true, blocked));
  assert.doesNotThrow(() => rememberShanty(false, blocked));
  assert.doesNotThrow(() => rememberShanty(true, null));
});

test("painting the mode touches only its own class, never VitePress's dark", () => {
  const classes = new Set(["dark", "preboot"]);
  const root = { classList: { add: (c: string) => classes.add(c), remove: (c: string) => classes.delete(c) } };
  paintShanty(root, true);
  assert.deepEqual([...classes].sort(), ["dark", "preboot", SHANTY_CLASS]);
  paintShanty(root, true);
  assert.equal(classes.size, 3);
  paintShanty(root, false);
  assert.deepEqual([...classes].sort(), ["dark", "preboot"]);
  paintShanty(root, false);
  assert.equal(classes.size, 2);
});

test("stripping the parameter keeps the other parameters, in order", () => {
  assert.equal(stripShantyParam(""), "");
  assert.equal(stripShantyParam("?shanty"), "");
  assert.equal(stripShantyParam("?shanty=1"), "");
  assert.equal(stripShantyParam("?a=1&shanty&b=2"), "?a=1&b=2");
  assert.equal(stripShantyParam("?a=1&SHANTY=off"), "?a=1");
  assert.equal(stripShantyParam("?shantytown=1&x=shanty"), "?shantytown=1&x=shanty");
  assert.equal(stripShantyParam("?a=1"), "?a=1");
});

test("the player shows the music video only where one was rendered", () => {
  const cases: [boolean, boolean, boolean, ReturnType<typeof reelKind>][] = [
    // on, showreel, video, result
    [false, true, true, "showreel"],
    [false, true, false, "showreel"],
    [false, false, true, null],
    [false, false, false, null],
    [true, true, true, "shanty"],
    [true, true, false, "showreel"],
    [true, false, true, "shanty"],
    [true, false, false, null],
  ];
  for (const [on, showreel, video, expected] of cases) {
    assert.equal(reelKind(on, { showreel, video }), expected, `on=${on} showreel=${showreel} video=${video}`);
  }
});

test("the switch announces its new state", () => {
  assert.equal(announcement(true), "Sea shanty mode on. This page is now in pirate.");
  assert.equal(announcement(true, false), "Sea shanty mode on. This page has no pirate version yet.");
  assert.equal(announcement(false), "Sea shanty mode off.");
  assert.equal(announcement(false, false), "Sea shanty mode off.");
});

test("a page and its pirate variant map onto each other", () => {
  const pairs: [string, string][] = [
    ["/", "/pirate/"],
    ["/hooks.html", "/pirate/hooks.html"],
    ["/hooks", "/pirate/hooks"],
    ["/cli/", "/pirate/cli/"],
    ["/cli/run/pre-commit.html", "/pirate/cli/run/pre-commit.html"],
    ["/reference/examples/", "/pirate/reference/examples/"],
  ];
  for (const [english, pirate] of pairs) {
    assert.equal(piratePath(english), pirate);
    assert.equal(englishPath(pirate), english);
    assert.equal(piratePath(pirate), pirate, "a pirate path stays put");
    assert.equal(englishPath(english), english, "an English path stays put");
    assert.equal(isPiratePath(pirate), true, pirate);
    assert.equal(isPiratePath(english), false, english);
  }
  assert.equal(englishPath("/pirate"), "/");
  for (const other of ["/pirates.html", "/piratey/", "/shanty.html", "/cli/pirate/"]) assert.equal(isPiratePath(other), false, other);
});

test("paths and Markdown files name the same page", () => {
  const cases: [string[], string, string][] = [
    // paths, Markdown file, key
    [["/", "/index.html", "/pirate/", "/pirate/index.html"], "index.md", ""],
    [["/hooks", "/hooks.html", "/pirate/hooks.html"], "hooks.md", "hooks"],
    [["/cli/", "/cli/index.html", "/pirate/cli/"], "cli/index.md", "cli/"],
    [["/cli/run/pre-commit.html"], "cli/run/pre-commit.md", "cli/run/pre-commit"],
  ];
  for (const [paths, file, key] of cases) {
    assert.equal(fileKey(file), key, file);
    for (const path of paths) assert.equal(pageKey(path), key, path);
  }
  assert.equal(hasVariant("/hooks.html", []), true);
  assert.equal(hasVariant("/hooks.html", ["hooks"]), false);
  assert.equal(hasVariant("/cli/", ["cli/"]), false);
  assert.equal(hasVariant("/cli/check.html", ["cli/"]), true);
});

/** Locations a page load can start from. */
const LOCATIONS = [
  "/",
  "/index.html",
  "/hooks.html",
  "/hooks",
  "/cli/",
  "/ci.html",
  "/pirate/",
  "/pirate/hooks.html",
  "/pirate/ci.html",
].flatMap((pathname) =>
  ["", "?shanty", "?shanty=0", "?a=1&shanty=off&b=2", "?a=1"].flatMap((search) =>
    ["", "#file-selection"].map((hash) => ({ pathname, search, hash })),
  ),
);
/** English pages without a variant in these tests. */
const MISSING = ["ci"];

test("a page load goes to the pirate variant only when the visitor wants the mode", () => {
  const on = store({ [SHANTY_KEY]: "1" });
  const off = store();
  assert.deepEqual(arrive({ pathname: "/hooks.html", search: "", hash: "" }, off, MISSING), { redirect: null, themed: false });
  assert.deepEqual(arrive({ pathname: "/hooks.html", search: "", hash: "#x" }, on, MISSING), { redirect: "/pirate/hooks.html#x", themed: true });
  assert.deepEqual(arrive({ pathname: "/", search: "?shanty", hash: "" }, off, MISSING), { redirect: "/pirate/", themed: true });
  assert.deepEqual(arrive({ pathname: "/hooks.html", search: "?a=1&shanty&b=2", hash: "" }, off, MISSING), {
    redirect: "/pirate/hooks.html?a=1&b=2",
    themed: true,
  });
  assert.deepEqual(arrive({ pathname: "/hooks.html", search: "?shanty=0", hash: "" }, on, MISSING), { redirect: null, themed: false });
  // No variant: the English page stays, themed.
  assert.deepEqual(arrive({ pathname: "/ci.html", search: "", hash: "" }, on, MISSING), { redirect: null, themed: true });
  // A pirate page is the mode, whatever was saved, until the URL says otherwise.
  assert.deepEqual(arrive({ pathname: "/pirate/hooks.html", search: "", hash: "" }, off, MISSING), { redirect: null, themed: true });
  assert.deepEqual(arrive({ pathname: "/pirate/hooks.html", search: "?shanty=no", hash: "#x" }, on, MISSING), {
    redirect: "/hooks.html#x",
    themed: false,
  });
  // Blocked storage is a visitor who never chose.
  assert.deepEqual(arrive({ pathname: "/hooks.html", search: "", hash: "" }, store({ [SHANTY_KEY]: "1" }, true), MISSING), {
    redirect: null,
    themed: false,
  });
});

test("after a navigation, pirate pages and untranslated pages with the mode on are themed", () => {
  assert.equal(themedAfterNavigation("/pirate/hooks.html", false, MISSING), true);
  assert.equal(themedAfterNavigation("/hooks.html", true, MISSING), false, "it will be taken to its variant");
  assert.equal(themedAfterNavigation("/ci.html", true, MISSING), true);
  assert.equal(themedAfterNavigation("/ci.html", false, MISSING), false);
});

/** Runs the pre-paint script as a browser would; returns where it went and whether it themed the page. */
function prePaint(
  where: { pathname: string; search: string; hash: string },
  stored: string | null,
  missing: string[],
  blocked = false,
): { redirect: string | null; themed: boolean } {
  const classes = new Set<string>();
  let redirect: string | null = null;
  const sandbox: Record<string, unknown> = {
    location: { ...where, replace: (to: string) => void (redirect = to) },
    document: { documentElement: { classList: { add: (c: string) => classes.add(c) } } },
  };
  Object.defineProperty(sandbox, "localStorage", {
    get() {
      if (blocked) throw new Error("blocked");
      return { getItem: (key: string) => (key === SHANTY_KEY ? stored : null) };
    },
  });
  vm.runInNewContext(prePaintScript(missing), sandbox);
  // A redirect leaves the page before it paints: whether it is themed does not matter.
  return { redirect, themed: redirect === null ? classes.has(SHANTY_CLASS) : isPiratePath(redirect) };
}

test("the pre-paint script reaches the same answer as arrive", () => {
  for (const where of LOCATIONS) {
    for (const stored of [null, "1", "0"]) {
      for (const blocked of [false, true]) {
        const expected = arrive(where, blocked ? store(stored ? { [SHANTY_KEY]: stored } : {}, true) : store(stored ? { [SHANTY_KEY]: stored } : {}), MISSING);
        assert.deepEqual(prePaint(where, stored, MISSING, blocked), expected, `${JSON.stringify(where)} stored=${stored} blocked=${blocked}`);
      }
    }
  }
});

test("the pre-paint script can sit inside a <script> element", () => {
  assert.ok(!prePaintScript(["a", "b/"]).includes("</"));
  assert.ok(!prePaintScript([]).includes("</"));
});

/** Reads the custom properties declared in the first block for `selector`. */
function tokens(css: string, selector: string): Record<string, string> {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\:]/g, "\\$&");
  const block = new RegExp(`(?:^|\\n)${escaped}\\s*\\{([^}]*)\\}`).exec(css.replace(/\/\*[\s\S]*?\*\//g, ""));
  assert.ok(block, `no block for ${selector}`);
  const out: Record<string, string> = {};
  for (const declaration of block[1].split(";")) {
    const at = declaration.indexOf(":");
    if (at > 0) out[declaration.slice(0, at).trim()] = declaration.slice(at + 1).trim();
  }
  return out;
}

type Rgb = [number, number, number];

/** A hex colour, or rgb(r g b / a%) laid over `under`. */
function color(value: string, under?: Rgb): Rgb {
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(value);
  if (hex) {
    const full = hex[1].length === 3 ? [...hex[1]].map((c) => c + c).join("") : hex[1];
    return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16)) as Rgb;
  }
  const rgb = /^rgb\((\d+) (\d+) (\d+) \/ (\d+)%\)$/.exec(value);
  assert.ok(rgb && under, `cannot read the colour ${value}`);
  const alpha = Number(rgb[4]) / 100;
  return [1, 2, 3].map((i) => Number(rgb[i]) * alpha + under[i - 1] * (1 - alpha)) as Rgb;
}

const luminance = ([r, g, b]: Rgb) => {
  const [lr, lg, lb] = [r, g, b].map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
};

const contrast = (a: Rgb, b: Rgb) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

const theme = join(REPO, "docs/.vitepress/theme");
const base = readFileSync(join(theme, "style.css"), "utf8");
const sea = readFileSync(join(theme, "shanty-mode.css"), "utf8");

/** The two palettes as the page resolves them: style.css, then the mode's overrides. */
const PALETTES = {
  light: { ...tokens(base, ":root"), ...tokens(sea, ":root:not(.dark).shanty-mode") },
  dark: { ...tokens(base, ":root"), ...tokens(base, ".dark"), ...tokens(sea, ":root.dark.shanty-mode") },
};

for (const [name, t] of Object.entries(PALETTES)) {
  const at = (token: string, under?: Rgb) => {
    assert.ok(t[token], `${name}: ${token} is not set`);
    return color(t[token], under);
  };

  test(`${name}: text reads on every surface the landing page uses`, () => {
    for (const surface of ["--vp-c-bg", "--vp-c-bg-alt", "--vp-c-bg-soft", "--vp-c-bg-elv"]) {
      for (const text of ["--vp-c-text-1", "--vp-c-text-2", "--vp-c-text-3"]) {
        const ratio = contrast(at(text), at(surface));
        assert.ok(ratio >= 4.5, `${name}: ${text} on ${surface} is ${ratio.toFixed(2)}:1`);
      }
    }
  });

  test(`${name}: links, buttons and code keep AA contrast`, () => {
    const bg = at("--vp-c-bg");
    for (const surface of ["--vp-c-bg", "--vp-c-bg-soft"]) {
      const ratio = contrast(at("--vp-c-brand-1"), at(surface));
      assert.ok(ratio >= 4.5, `${name}: brand-1 on ${surface} is ${ratio.toFixed(2)}:1`);
    }
    // The pill's hover tint and the lane chips sit on the page background.
    assert.ok(contrast(at("--vp-c-brand-1"), at("--vp-c-brand-soft", bg)) >= 4.5, `${name}: brand-1 on brand-soft`);
    assert.ok(contrast(at("--hk-code-keyword"), at("--hk-write-bg", bg)) >= 4.5, `${name}: code keyword on write-bg`);
    for (const fill of ["--vp-c-brand-2", "--vp-c-brand-3"]) {
      const ratio = contrast(at("--vp-button-brand-text"), at(fill));
      assert.ok(ratio >= 4.5, `${name}: button text on ${fill} is ${ratio.toFixed(2)}:1`);
    }
  });

  test(`${name}: brass and borders are legible`, () => {
    assert.ok(contrast(at("--hk-sea-brass"), at("--vp-c-bg")) >= 4.5, `${name}: brass on bg`);
    assert.ok(contrast(at("--vp-c-border"), at("--vp-c-bg")) >= 3, `${name}: border on bg`);
  });
}

test("the light palette cannot leak into the dark one", () => {
  assert.match(sea, /:root:not\(\.dark\)\.shanty-mode\s*\{/);
  assert.match(sea, /:root\.dark\.shanty-mode\s*\{/);
});
