// Sea shanty mode: the whole site as the shanty sings it. Every English page
// has a pirate variant under /pirate/ (a VitePress locale), and the switch in
// the header moves between the two. The choice is remembered in localStorage,
// so a visitor who turned the mode on is taken to the pirate variant of any
// page they open, and `?shanty` offers the mode in a shared link. While the
// mode is on, <html> carries one class that shanty-mode.css themes the site
// from. This file holds the rules and nothing else: no Vue, and no DOM or
// storage access until a function is called, so it is safe to server-render,
// to import from config.mts, and to test with plain objects.

/** On <html> while the mode is on. */
export const SHANTY_CLASS = "shanty-mode";
/** On <html> while the switch is playing its ripple. */
export const TURNING_CLASS = "shanty-turning";
/** localStorage key; "1" when the visitor turned the mode on. */
export const SHANTY_KEY = "hk-shanty-mode";
/** The VitePress locale that holds the pirate pages, and its path prefix. */
export const PIRATE_LOCALE = "pirate";
export const PIRATE_PREFIX = `/${PIRATE_LOCALE}/`;

/**
 * The `?shanty` parameter: on when bare or given any value, off for 0, off,
 * false and no. `?shantytown` and `?x=shanty` are other parameters. The
 * pre-paint script embeds this pattern, so the two cannot disagree.
 */
const PARAM_RE = /(?:^|[?&])shanty(?:=([^&#]*))?(?=&|#|$)/i;
const OFF_VALUE_RE = /^(?:0|off|false|no)$/i;
const PIRATE_RE = /^\/pirate(?:\/|$)/;

/** What a URL's query says: true, false, or null when it does not mention the mode. */
export function paramState(search: string): boolean | null {
  const match = PARAM_RE.exec(search);
  return match ? !OFF_VALUE_RE.test(match[1] ?? "") : null;
}

/** The subset of Storage the mode reads and writes. */
export type ShantyStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

/** localStorage, or null where reaching for it throws (blocked site data). */
export function browserStorage(): ShantyStorage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** Whether the visitor turned the mode on last time. A blocked store says no. */
export function storedShanty(storage: Pick<Storage, "getItem"> | null | undefined): boolean {
  try {
    return storage?.getItem(SHANTY_KEY) === "1";
  } catch {
    return false;
  }
}

/** Whether a visit wants the mode: the URL decides, else the visitor's last choice. */
export function wantsShanty(search: string, storage: Pick<Storage, "getItem"> | null | undefined): boolean {
  return paramState(search) ?? storedShanty(storage);
}

/** Remembers the visitor's choice. A blocked store keeps it for this visit only. */
export function rememberShanty(on: boolean, storage: Pick<Storage, "setItem" | "removeItem"> | null | undefined): void {
  try {
    if (on) storage?.setItem(SHANTY_KEY, "1");
    else storage?.removeItem(SHANTY_KEY);
  } catch {
    // The mode still works for this visit.
  }
}

/** Sets or clears the mode's class on <html>. Only that class: `dark` is VitePress's. */
export function paintShanty(root: { classList: Pick<DOMTokenList, "add" | "remove"> }, on: boolean): void {
  if (on) root.classList.add(SHANTY_CLASS);
  else root.classList.remove(SHANTY_CLASS);
}

/** A query string without the `?shanty` parameter, the others in their order. */
export function stripShantyParam(search: string): string {
  const kept = search
    .replace(/^\?/, "")
    .split("&")
    .filter((part) => part && !/^shanty(?:=.*)?$/i.test(part));
  return kept.length ? `?${kept.join("&")}` : "";
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
 * The page a path shows, as `pageKey` of its English Markdown file: `hooks`
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

/** What a page load does before its first paint. */
export interface Arrival {
  /** Where to go instead, keeping the query and fragment, or null to stay. */
  redirect: string | null;
  /** Whether the page is themed while it stays. */
  themed: boolean;
}

/**
 * Decides a page load: a pirate page is themed, and `?shanty=0` takes it back
 * to English. An English page whose visitor wants the mode goes to its pirate
 * variant; one with no variant stays and is themed in place. The pre-paint
 * script is this function written out for <head>; the tests hold them to the
 * same answers.
 */
export function arrive(
  location: { pathname: string; search: string; hash: string },
  storage: Pick<Storage, "getItem"> | null | undefined,
  missing: readonly string[],
): Arrival {
  const { pathname, search, hash } = location;
  const fromUrl = paramState(search);
  if (isPiratePath(pathname)) {
    return fromUrl === false
      ? { redirect: englishPath(pathname) + stripShantyParam(search) + hash, themed: false }
      : { redirect: null, themed: true };
  }
  if (!(fromUrl ?? storedShanty(storage))) return { redirect: null, themed: false };
  if (!hasVariant(pathname, missing)) return { redirect: null, themed: true };
  return { redirect: piratePath(pathname) + stripShantyParam(search) + hash, themed: true };
}

/**
 * Whether a page shown after a navigation inside the site is themed: every
 * pirate page, and an English page with no variant while the visitor has the
 * mode on.
 */
export function themedAfterNavigation(pathname: string, on: boolean, missing: readonly string[]): boolean {
  return isPiratePath(pathname) || (on && !hasVariant(pathname, missing));
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
 * Runs in <head> of every page before the first paint, so a visitor who chose
 * the mode is taken to the pirate variant without seeing the English page,
 * and a themed page never flashes plain. It is `arrive` written out for a
 * browser; the class is on <html>, outside the app, so hydration does not see
 * it. `missing` lists the English pages with no pirate variant. It holds no
 * `</`.
 */
export function prePaintScript(missing: readonly string[]): string {
  return `(function () {
  try {
    var l = location, p = l.pathname, s = l.search;
    var m = /${PARAM_RE.source}/i.exec(s);
    var url = m ? !/${OFF_VALUE_RE.source}/i.test(m[1] || "") : null;
    var pirate = ${PIRATE_RE}.test(p);
    var strip = function () {
      var kept = s.replace(/^\\?/, "").split("&").filter(function (x) { return x && !/^shanty(?:=.*)?$/i.test(x); });
      return (kept.length ? "?" + kept.join("&") : "") + l.hash;
    };
    var on = url;
    if (on === null && !pirate) {
      try { on = localStorage.getItem(${JSON.stringify(SHANTY_KEY)}) === "1"; } catch (e) { on = false; }
    }
    if (pirate) {
      if (on === false) return l.replace(p.replace(${PIRATE_RE}, "/") + strip());
    } else {
      if (!on) return;
      var key = p.replace(/^\\//, "").replace(/(^|\\/)index(?:\\.html)?$/, "$1").replace(/\\.html$/, "");
      if (${JSON.stringify(missing)}.indexOf(key) < 0) return l.replace("/${PIRATE_LOCALE}" + p + strip());
    }
    document.documentElement.classList.add(${JSON.stringify(SHANTY_CLASS)});
  } catch (e) {}
})();`;
}
