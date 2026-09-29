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
  type Choice,
  englishPath,
  fileKey,
  hasVariant,
  isPiratePath,
  modeOn,
  pageKey,
  paintShanty,
  paramState,
  pirateLink,
  piratePath,
  prePaintScript,
  readChoice,
  reelKind,
  routeFor,
  SHANTY_CLASS,
  SHANTY_KEY,
  stripShantyParam,
  themedPage,
  VISIT_KEY,
  visitFromAddress,
  writeChoice,
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

test("stripping the parameter keeps the other parameters, in order", () => {
  assert.equal(stripShantyParam(""), "");
  assert.equal(stripShantyParam("?shanty"), "");
  assert.equal(stripShantyParam("?shanty=1"), "");
  assert.equal(stripShantyParam("?a=1&shanty&b=2"), "?a=1&b=2");
  assert.equal(stripShantyParam("?a=1&SHANTY=off"), "?a=1");
  assert.equal(stripShantyParam("?shantytown=1&x=shanty"), "?shantytown=1&x=shanty");
  assert.equal(stripShantyParam("?a=1"), "?a=1");
});

test("a stored choice is 1 or 0; anything else, or a blocked store, is no choice", () => {
  assert.equal(readChoice(store({ k: "1" }), "k"), true);
  assert.equal(readChoice(store({ k: "0" }), "k"), false);
  for (const other of ["true", "on", "", "yes", " 1", "01"]) assert.equal(readChoice(store({ k: other }), "k"), null, JSON.stringify(other));
  assert.equal(readChoice(store(), "k"), null);
  assert.equal(readChoice(store({ k: "1" }, true), "k"), null);
  assert.equal(readChoice(null, "k"), null);
  assert.equal(readChoice(undefined, "k"), null);
});

test("choices are stored, forgotten, and survive a blocked store", () => {
  const s = store();
  writeChoice(s, "k", true);
  assert.equal(s.values.get("k"), "1");
  writeChoice(s, "k", false);
  assert.equal(s.values.get("k"), "0");
  writeChoice(s, "k", null);
  assert.equal(s.values.has("k"), false);
  assert.doesNotThrow(() => writeChoice(store({}, true), "k", true));
  assert.doesNotThrow(() => writeChoice(null, "k", false));
});

test("the visit's choice beats the saved one, which beats off", () => {
  const cases: [Choice, Choice, boolean][] = [
    [null, null, false],
    [null, true, true],
    [null, false, false],
    [true, false, true],
    [false, true, false],
    [true, null, true],
    [false, null, false],
  ];
  for (const [visit, saved, expected] of cases) assert.equal(modeOn(visit, saved), expected, `visit=${visit} saved=${saved}`);
});

test("an opened address chooses for the visit; returning to a page does not", () => {
  assert.equal(visitFromAddress("/hooks.html", "?shanty", "navigate"), true);
  assert.equal(visitFromAddress("/hooks.html", "?shanty=0", "navigate"), false);
  assert.equal(visitFromAddress("/pirate/hooks.html", "?shanty=off", "navigate"), false);
  assert.equal(visitFromAddress("/pirate/hooks.html", "", "navigate"), true);
  assert.equal(visitFromAddress("/pirate/hooks.html", "", "reload"), true);
  assert.equal(visitFromAddress("/pirate/hooks.html", "", "back_forward"), null);
  assert.equal(visitFromAddress("/pirate/hooks.html", "?shanty=0", "back_forward"), false, "the parameter still speaks");
  assert.equal(visitFromAddress("/hooks.html", "", "navigate"), null);
  assert.equal(visitFromAddress("/hooks.html", "?other", "navigate"), null);
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
  assert.equal(englishPath("/pirate.html"), "/");
  for (const other of ["/pirates.html", "/piratey/", "/shanty.html", "/cli/pirate/", "/pirate.htmlx"]) {
    assert.equal(isPiratePath(other), false, other);
  }
});

test("paths and Markdown files name the same page", () => {
  const cases: [string[], string, string][] = [
    // paths, Markdown file, key
    [["/", "/index.html", "/pirate/", "/pirate/index.html", "/pirate", "/pirate.html"], "index.md", ""],
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

test("links from pirate pages go to variants where there are any", () => {
  assert.equal(pirateLink("/hooks#stashing", []), "/pirate/hooks#stashing");
  assert.equal(pirateLink("/getting_started?x=1#install", []), "/pirate/getting_started?x=1#install");
  assert.equal(pirateLink("/why-hk", ["why-hk"]), "/why-hk");
  assert.equal(pirateLink("/", []), "/pirate/");
});

/** English pages without a variant in these tests. */
const MISSING = ["ci"];

test("each page is shown where the mode says it belongs", () => {
  const cases: [string, boolean, string | null][] = [
    ["/hooks.html", true, "/pirate/hooks.html"],
    ["/hooks.html", false, null],
    ["/pirate/hooks.html", true, null],
    ["/pirate/hooks.html", false, "/hooks.html"],
    ["/ci.html", true, null],
    ["/ci.html", false, null],
    ["/pirate/ci.html", false, "/ci.html"],
    ["/", true, "/pirate/"],
    ["/pirate/", false, "/"],
    ["/pirate", true, "/pirate/"],
    ["/pirate.html", true, "/pirate/"],
    ["/pirate.html", false, "/"],
  ];
  for (const [path, on, expected] of cases) assert.equal(routeFor(path, on, MISSING), expected, `${path} on=${on}`);
  assert.equal(themedPage("/pirate/hooks.html", false, MISSING), true);
  assert.equal(themedPage("/hooks.html", true, MISSING), false);
  assert.equal(themedPage("/ci.html", true, MISSING), true);
  assert.equal(themedPage("/ci.html", false, MISSING), false);
});

test("page loads: the address, the visit and the saved choice, in that order", () => {
  const load = (href: string, visit: Choice, saved: Choice, navigation = "navigate") => {
    const url = new URL(href, "https://hk.jdx.dev");
    return arrive({ pathname: url.pathname, search: url.search, hash: url.hash }, { visit, saved }, navigation, MISSING);
  };
  assert.deepEqual(load("/hooks.html", null, null), { redirect: null, themed: false, visit: null });
  assert.deepEqual(load("/hooks.html#x", null, true), { redirect: "/pirate/hooks.html#x", themed: true, visit: null });
  assert.deepEqual(load("/?shanty", null, null), { redirect: "/pirate/", themed: true, visit: true });
  assert.deepEqual(load("/hooks.html?a=1&shanty&b=2", null, false), { redirect: "/pirate/hooks.html?a=1&b=2", themed: true, visit: true });
  assert.deepEqual(load("/hooks.html?shanty=0", null, true), { redirect: null, themed: false, visit: false });
  // The visit's choice outlasts the address that made it.
  assert.deepEqual(load("/hooks.html", false, true), { redirect: null, themed: false, visit: false });
  assert.deepEqual(load("/hooks.html", true, false), { redirect: "/pirate/hooks.html", themed: true, visit: true });
  // A pirate address opens the mode for the visit, whatever was saved…
  assert.deepEqual(load("/pirate/hooks.html", null, false), { redirect: null, themed: true, visit: true });
  // …but returning to one with Back follows the choices in force.
  assert.deepEqual(load("/pirate/hooks.html", null, false, "back_forward"), { redirect: "/hooks.html", themed: false, visit: null });
  assert.deepEqual(load("/pirate/hooks.html?shanty=no#x", null, true), { redirect: "/hooks.html#x", themed: false, visit: false });
  // No variant: the English page stays, themed while the mode is on.
  assert.deepEqual(load("/ci.html", null, true), { redirect: null, themed: true, visit: null });
  assert.deepEqual(load("/ci.html?shanty", null, null), { redirect: null, themed: true, visit: true });
});

/** Locations a page load can start from. */
const LOCATIONS = [
  "/",
  "/index.html",
  "/hooks.html",
  "/hooks",
  "/cli/",
  "/ci.html",
  "/pirate",
  "/pirate.html",
  "/pirate/",
  "/pirate/hooks.html",
  "/pirate/ci.html",
].flatMap((pathname) =>
  ["", "?shanty", "?shanty=0", "?a=1&shanty=off&b=2", "?a=1"].flatMap((search) =>
    ["", "#file-selection"].map((hash) => ({ pathname, search, hash })),
  ),
);
const CHOICES: Choice[] = [null, true, false];
const NAVIGATIONS = ["navigate", "reload", "back_forward"];
const encode = (choice: Choice) => (choice === null ? null : choice ? "1" : "0");

/** Runs the pre-paint script as a browser would. */
function prePaint(
  where: { pathname: string; search: string; hash: string },
  stores: { visit: Choice; saved: Choice },
  navigation: string,
  blocked: { local?: boolean; session?: boolean } = {},
) {
  const classes = new Set<string>();
  let redirect: string | null = null;
  const session = store(encode(stores.visit) === null ? {} : { [VISIT_KEY]: encode(stores.visit)! }, blocked.session);
  const local = store(encode(stores.saved) === null ? {} : { [SHANTY_KEY]: encode(stores.saved)! }, blocked.local);
  const sandbox: Record<string, unknown> = {
    location: { ...where, replace: (to: string) => void (redirect = to) },
    document: { documentElement: { classList: { add: (c: string) => classes.add(c) } } },
    performance: { getEntriesByType: (kind: string) => (kind === "navigation" ? [{ type: navigation }] : []) },
  };
  for (const [name, backing, isBlocked] of [
    ["localStorage", local, blocked.local],
    ["sessionStorage", session, blocked.session],
  ] as const) {
    Object.defineProperty(sandbox, name, {
      get() {
        if (isBlocked) throw new Error("blocked");
        return backing;
      },
    });
  }
  sandbox.window = sandbox;
  vm.runInNewContext(prePaintScript(MISSING), sandbox);
  return {
    redirect: redirect as string | null,
    themed: redirect === null ? classes.has(SHANTY_CLASS) : isPiratePath(redirect),
    visit: readChoice(blocked.session ? store() : session, VISIT_KEY),
  };
}

test("the pre-paint script reaches the same answer as arrive", () => {
  for (const where of LOCATIONS) {
    for (const visit of CHOICES) {
      for (const saved of CHOICES) {
        for (const navigation of NAVIGATIONS) {
          const expected = arrive(where, { visit, saved }, navigation, MISSING);
          const label = `${JSON.stringify(where)} visit=${visit} saved=${saved} ${navigation}`;
          assert.deepEqual(prePaint(where, { visit, saved }, navigation), expected, label);
          // Blocked stores read as no choice, and the address still decides.
          const blind = arrive(where, { visit: null, saved: null }, navigation, MISSING);
          const got = prePaint(where, { visit, saved }, navigation, { local: true, session: true });
          assert.deepEqual({ redirect: got.redirect, themed: got.themed }, { redirect: blind.redirect, themed: blind.themed }, `${label} blocked`);
        }
      }
    }
  }
});

test("a page load redirects at most once, and lands where the mode says", () => {
  for (const start of LOCATIONS) {
    for (const visit of CHOICES) {
      for (const saved of CHOICES) {
        for (const navigation of NAVIGATIONS) {
          let where = start;
          let stores = { visit, saved };
          let result = prePaint(where, stores, navigation);
          const hops = [where.pathname + where.search];
          if (result.redirect) {
            // location.replace is a fresh navigation, with the visit the first load stored.
            const url = new URL(result.redirect, "https://hk.jdx.dev");
            where = { pathname: url.pathname, search: url.search, hash: url.hash };
            stores = { visit: result.visit, saved };
            hops.push(where.pathname + where.search);
            result = prePaint(where, stores, "navigate");
          }
          const label = `${hops.join(" -> ")} visit=${visit} saved=${saved} ${navigation}`;
          assert.equal(result.redirect, null, `${label} redirects again`);
          assert.equal(result.themed, modeOn(result.visit, saved) || isPiratePath(where.pathname), label);
          assert.equal(where.hash, start.hash, `${label} keeps the fragment`);
        }
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
    // The sidebar's headings are brass on the sidebar's parchment, and the X
    // that marks the page you are on is a mark, so 3:1.
    assert.ok(contrast(at("--hk-sea-brass"), at("--vp-c-bg-alt")) >= 4.5, `${name}: brass on bg-alt`);
    for (const surface of ["--vp-c-bg", "--vp-c-bg-alt"]) {
      assert.ok(contrast(at("--hk-sea-mark"), at(surface)) >= 3, `${name}: mark on ${surface}`);
    }
  });

  test(`${name}: each kind of note reads on its own paper`, () => {
    const bg = at("--vp-c-bg");
    for (const kind of ["tip", "info", "warning", "danger"]) {
      const paper = at(`--hk-sea-${kind}-bg`, bg);
      for (const text of [`--hk-sea-${kind}`, "--vp-c-text-1", "--vp-c-text-2"]) {
        const ratio = contrast(at(text), paper);
        assert.ok(ratio >= 4.5, `${name}: ${text} on the ${kind} note is ${ratio.toFixed(2)}:1`);
      }
    }
  });

  test(`${name}: code, stamps and signposts keep AA contrast`, () => {
    const bg = at("--vp-c-bg");
    const card = at("--vp-code-block-bg");
    // Inline code is brand teal on its stamp; a code card's language is brass.
    assert.ok(contrast(at("--vp-c-brand-1"), at("--vp-code-bg", bg)) >= 4.5, `${name}: inline code on its stamp`);
    assert.ok(contrast(at("--hk-sea-brass"), card) >= 4.5, `${name}: brass on a code card`);
    // The Shiki colours the docs use, with the two shanty-mode.css replaces.
    const shiki =
      name === "light"
        ? ["#24292e", "#032f62", "#005cc5", "#6f42c1", "#6a737d", t["--hk-sea-code-red"]]
        : ["#e1e4e8", "#9ecbff", "#f97583", "#79b8ff", "#b392f0", "#dbedff", t["--hk-sea-code-comment"]];
    for (const token of shiki) {
      const ratio = contrast(color(token), card);
      assert.ok(ratio >= 4.5, `${name}: code colour ${token} on a code card is ${ratio.toFixed(2)}:1`);
    }
    // The signposts at a page's foot: brass "Astern" over a brand-teal title.
    for (const text of ["--hk-sea-brass", "--vp-c-brand-1"]) {
      const ratio = contrast(at(text), at("--hk-sea-post"));
      assert.ok(ratio >= 4.5, `${name}: ${text} on a signpost is ${ratio.toFixed(2)}:1`);
    }
  });
}

test("the light palette cannot leak into the dark one", () => {
  assert.match(sea, /:root:not\(\.dark\)\.shanty-mode\s*\{/);
  assert.match(sea, /:root\.dark\.shanty-mode\s*\{/);
});
