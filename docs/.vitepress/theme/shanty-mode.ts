// Sea shanty mode: the whole site as the shanty sings it. Every English page
// has a pirate variant under /pirate/ (a VitePress locale), and the switch in
// the header moves between the two. While the mode is on, <html> carries one
// class that shanty-mode.css themes the site from. This file holds the rules
// and nothing else: no Vue, and no DOM or storage access until a function is
// called, so it is safe to server-render, to import from config.mts, and to
// test with plain objects.
//
// Whether the mode is on comes from two choices:
//
// - The switch saves the visitor's choice in localStorage, for every visit.
// - An address the visitor opens decides for the rest of the visit (the tab,
//   in sessionStorage) without saving anything: `?shanty` turns the mode on,
//   `?shanty=0` turns it off, and a pirate page's address turns it on, so a
//   shared pirate link stays pirate as the visitor follows its links. Using
//   the switch ends the visit's choice, and the saved one applies again.
//
// With the mode on, every page that has a variant is shown as the variant,
// including pages reached with Back and Forward; with it off, every pirate
// page is shown as its English page. An English page with no variant is shown
// in English, themed while the mode is on.

/** On <html> while the mode is on. */
export const SHANTY_CLASS = "shanty-mode";
/** On <html> while the switch is playing its ripple. */
export const TURNING_CLASS = "shanty-turning";
/** localStorage: the switch's choice, "1" on and "0" off; absent until the visitor uses it. */
export const SHANTY_KEY = "hk-shanty-mode";
/** sessionStorage: the choice an address made for this visit, "1" or "0"; absent when none did. */
export const VISIT_KEY = "hk-shanty-visit";
/** The VitePress locale that holds the pirate pages, and its path prefix. */
export const PIRATE_LOCALE = "pirate";
export const PIRATE_PREFIX = `/${PIRATE_LOCALE}/`;

/**
 * The `?shanty` parameter: on when bare or given any value, off for 0, off,
 * false and no. `?shantytown` and `?x=shanty` are other parameters. The
 * pre-paint script embeds these patterns, so the two cannot disagree.
 */
const PARAM_RE = /(?:^|[?&])shanty(?:=([^&#]*))?(?=&|#|$)/i;
const OFF_VALUE_RE = /^(?:0|off|false|no)$/i;
/** A pirate page's path, including the landing page's /pirate and /pirate.html forms. */
const PIRATE_RE = /^\/pirate(?:\/|\.html$|$)/;
/** The pirate landing page's path without its slash, which VitePress would load as a page of its own. */
const PIRATE_ROOT_RE = /^\/pirate(?:\.html)?$/;

/** What a URL's query says: true, false, or null when it does not mention the mode. */
export function paramState(search: string): boolean | null {
  const match = PARAM_RE.exec(search);
  return match ? !OFF_VALUE_RE.test(match[1] ?? "") : null;
}

/** A query string without the `?shanty` parameter, the others in their order. */
export function stripShantyParam(search: string): string {
  const kept = search
    .replace(/^\?/, "")
    .split("&")
    .filter((part) => part && !/^shanty(?:=.*)?$/i.test(part));
  return kept.length ? `?${kept.join("&")}` : "";
}

/** A choice: on, off, or not made. */
export type Choice = boolean | null;

/** The subset of Storage the mode reads and writes. */
export type ShantyStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

/** localStorage or sessionStorage, or null where reaching for it throws (blocked site data). */
export function browserStorage(kind: "localStorage" | "sessionStorage"): ShantyStorage | null {
  try {
    return window[kind];
  } catch {
    return null;
  }
}

/** The choice stored under `key`: "1" is on, "0" is off, anything else (or a blocked store) is none. */
export function readChoice(storage: Pick<Storage, "getItem"> | null | undefined, key: string): Choice {
  try {
    const value = storage?.getItem(key);
    return value === "1" ? true : value === "0" ? false : null;
  } catch {
    return null;
  }
}

/** Stores a choice under `key`, or forgets it for null. A blocked store keeps it for this page only. */
export function writeChoice(storage: Pick<Storage, "setItem" | "removeItem"> | null | undefined, key: string, choice: Choice): void {
  try {
    if (choice === null) storage?.removeItem(key);
    else storage?.setItem(key, choice ? "1" : "0");
  } catch {
    // The mode still works for this page.
  }
}

/** Whether the mode is on: the visit's choice, else the saved one, else off. */
export function modeOn(visit: Choice, saved: Choice): boolean {
  return (visit ?? saved) === true;
}

/**
 * What opening an address chooses for the visit: its `?shanty` parameter, or
 * on for a pirate page's address. Returning to a page with Back or Forward
 * opens no address, so it chooses nothing, and nor does an English address
 * without the parameter.
 */
export function visitFromAddress(pathname: string, search: string, navigation: string): Choice {
  const fromParam = paramState(search);
  if (fromParam !== null) return fromParam;
  return isPiratePath(pathname) && navigation !== "back_forward" ? true : null;
}

/** Sets or clears the mode's class on <html>. Only that class: `dark` is VitePress's. */
export function paintShanty(root: { classList: Pick<DOMTokenList, "add" | "remove"> }, on: boolean): void {
  if (on) root.classList.add(SHANTY_CLASS);
  else root.classList.remove(SHANTY_CLASS);
}

/** Whether a site path is one of the pirate pages. */
export function isPiratePath(pathname: string): boolean {
  return PIRATE_RE.test(pathname);
}

/** The pirate variant's path for an English page's path: `/hooks.html` → `/pirate/hooks.html`. */
export function piratePath(pathname: string): string {
  return isPiratePath(pathname) ? pathname : `/${PIRATE_LOCALE}${pathname.startsWith("/") ? "" : "/"}${pathname}`;
}

/** The English page's path for a pirate variant's path: `/pirate/hooks.html` → `/hooks.html`. */
export function englishPath(pathname: string): string {
  return isPiratePath(pathname) ? pathname.replace(PIRATE_RE, "/") : pathname;
}

/**
 * The page a path shows, as `fileKey` of its English Markdown file: `hooks`
 * for `/hooks`, `/hooks.html` and `/pirate/hooks.html`; `cli/` for `/cli/`;
 * and the empty string for the landing page.
 */
export function pageKey(pathname: string): string {
  return englishPath(pathname)
    .replace(/^\//, "")
    .replace(/(^|\/)index(?:\.html)?$/, "$1")
    .replace(/\.html$/, "");
}

/** The key of an English Markdown file, relative to docs/: `cli/index.md` → `cli/`. */
export function fileKey(relativePath: string): string {
  return relativePath.replace(/(^|\/)index\.md$/, "$1").replace(/\.md$/, "");
}

/** Whether an English page has a pirate variant, given the pages that do not. */
export function hasVariant(pathname: string, missing: readonly string[]): boolean {
  return !missing.includes(pageKey(pathname));
}

/**
 * A link to a site page from a pirate page: to its variant when it has one,
 * else to the English page. The query and fragment are kept.
 */
export function pirateLink(link: string, missing: readonly string[]): string {
  const [, path, rest] = /^([^?#]*)(.*)$/.exec(link)!;
  return hasVariant(path, missing) ? piratePath(path) + rest : link;
}

/**
 * Where a page is shown instead, or null when it is shown where it is: with
 * the mode on, an English page that has a variant is shown as the variant;
 * with it off, a pirate page is shown as its English page.
 */
export function routeFor(pathname: string, on: boolean, missing: readonly string[]): string | null {
  if (PIRATE_ROOT_RE.test(pathname)) return on ? PIRATE_PREFIX : "/";
  if (isPiratePath(pathname)) return on ? null : englishPath(pathname);
  return on && hasVariant(pathname, missing) ? piratePath(pathname) : null;
}

/** Whether a page shown where it is gets the theme: a pirate page, or an English page with no variant while the mode is on. */
export function themedPage(pathname: string, on: boolean, missing: readonly string[]): boolean {
  return isPiratePath(pathname) || (on && !hasVariant(pathname, missing));
}

/** What a page load does before its first paint. */
export interface Arrival {
  /** Where to go instead, with the other query parameters and the fragment, or null to stay. */
  redirect: string | null;
  /** Whether the page is themed while it stays. */
  themed: boolean;
  /** The visit's choice after this load, for sessionStorage. */
  visit: Choice;
}

/**
 * Decides a page load: the address may make the visit's choice, and the mode
 * then shows the page where it belongs. The pre-paint script is this function
 * written out for <head>; the tests hold them to the same answers, and to at
 * most one redirect.
 */
export function arrive(
  location: { pathname: string; search: string; hash: string },
  stored: { visit: Choice; saved: Choice },
  navigation: string,
  missing: readonly string[],
): Arrival {
  const { pathname, search, hash } = location;
  const visit = visitFromAddress(pathname, search, navigation) ?? stored.visit;
  const on = modeOn(visit, stored.saved);
  const to = routeFor(pathname, on, missing);
  return to === null
    ? { redirect: null, themed: themedPage(pathname, on, missing), visit }
    : { redirect: to + stripShantyParam(search) + hash, themed: isPiratePath(to), visit };
}

/**
 * Which video the landing page's player shows: the shanty's music video on
 * the pirate landing page when this build rendered it, else the showreel if
 * this build rendered that, else none. The player never shows a video that is
 * not there.
 */
export function reelKind(on: boolean, have: { showreel: boolean; video: boolean }): "showreel" | "shanty" | null {
  if (on && have.video) return "shanty";
  return have.showreel ? "showreel" : null;
}

/** What the switch tells a screen reader after it flips. */
export function announcement(on: boolean, hasVariantPage = true): string {
  if (!on) return "Sea shanty mode off.";
  return hasVariantPage
    ? "Sea shanty mode on. This page is now in pirate."
    : "Sea shanty mode on. This page has no pirate version yet.";
}

/**
 * Runs in <head> of every page before the first paint, so a visitor in the
 * mode is taken to the pirate variant without seeing the English page, and a
 * themed page never flashes plain. It is `arrive` written out for a browser,
 * and records the visit's choice for the pages that follow. The class is on
 * <html>, outside the app, so hydration does not see it. `missing` lists the
 * English pages with no pirate variant. It holds no `</`.
 */
export function prePaintScript(missing: readonly string[]): string {
  return `(function () {
  var l = location, p = l.pathname, s = l.search;
  var read = function (store, key) {
    try {
      var v = window[store].getItem(key);
      return v === "1" ? true : v === "0" ? false : null;
    } catch (e) {
      return null;
    }
  };
  var nav = "navigate";
  try {
    nav = performance.getEntriesByType("navigation")[0].type;
  } catch (e) {}
  var pirate = ${PIRATE_RE}.test(p);
  var m = /${PARAM_RE.source}/i.exec(s);
  var visit = m ? !/${OFF_VALUE_RE.source}/i.test(m[1] || "") : pirate && nav !== "back_forward" ? true : null;
  if (visit === null) visit = read("sessionStorage", ${JSON.stringify(VISIT_KEY)});
  else
    try {
      sessionStorage.setItem(${JSON.stringify(VISIT_KEY)}, visit ? "1" : "0");
    } catch (e) {}
  var on = (visit === null ? read("localStorage", ${JSON.stringify(SHANTY_KEY)}) : visit) === true;
  var key = p.replace(${PIRATE_RE}, "/").replace(/^\\//, "").replace(/(^|\\/)index(?:\\.html)?$/, "$1").replace(/\\.html$/, "");
  var variant = ${JSON.stringify(missing)}.indexOf(key) < 0;
  var to = ${PIRATE_ROOT_RE}.test(p)
    ? on ? ${JSON.stringify(PIRATE_PREFIX)} : "/"
    : pirate ? (on ? null : p.replace(${PIRATE_RE}, "/"))
    : on && variant ? "/${PIRATE_LOCALE}" + p : null;
  if (to !== null) {
    var kept = s.replace(/^\\?/, "").split("&").filter(function (x) { return x && !/^shanty(?:=.*)?$/i.test(x); });
    return l.replace(to + (kept.length ? "?" + kept.join("&") : "") + l.hash);
  }
  if (pirate || (on && !variant)) document.documentElement.classList.add(${JSON.stringify(SHANTY_CLASS)});
})();`;
}
