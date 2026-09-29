// Sea shanty mode: the landing page's switch between the plain page and the
// shanty's. The state is one class on <html> (shanty-mode.css themes the page
// from it), remembered in localStorage and offered by a `?shanty` link. This
// file holds the rules and nothing else: no Vue, and no DOM or storage access
// until a function is called, so it is safe to server-render, to import from
// config.mts, and to test with plain objects.

/** On <html> while the mode is on. */
export const SHANTY_CLASS = "shanty-mode";
/** On <html> while the switch is playing its ripple. */
export const TURNING_CLASS = "shanty-turning";
/** localStorage key; "1" when the visitor turned the mode on. */
export const SHANTY_KEY = "hk-shanty-mode";

/**
 * The `?shanty` parameter: on when bare or given any value, off for 0, off,
 * false and no. `?shantytown` and `?x=shanty` are other parameters. The
 * pre-paint script embeds this pattern, so the two cannot disagree.
 */
const PARAM_RE = /(?:^|[?&])shanty(?:=([^&#]*))?(?=&|#|$)/i;
const OFF_VALUE_RE = /^(?:0|off|false|no)$/i;

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

/** Whether a visit starts with the mode on: the URL decides, else the visitor's last choice. */
export function wantsShanty(search: string, storage: Pick<Storage, "getItem"> | null | undefined): boolean {
  const fromUrl = paramState(search);
  if (fromUrl !== null) return fromUrl;
  try {
    return storage?.getItem(SHANTY_KEY) === "1";
  } catch {
    return false;
  }
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

/**
 * Which video the landing page's player shows: the shanty's music video when
 * the mode is on and this build rendered it, else the showreel if this build
 * rendered that, else none. The player never shows a video that is not there.
 */
export function reelKind(on: boolean, have: { showreel: boolean; video: boolean }): "showreel" | "shanty" | null {
  if (on && have.video) return "shanty";
  return have.showreel ? "showreel" : null;
}

/** What the switch tells a screen reader after it flips. */
export function announcement(on: boolean, hasVideo: boolean): string {
  if (!on) return "Sea shanty mode off.";
  return hasVideo ? "Sea shanty mode on. The music video is below." : "Sea shanty mode on.";
}

/**
 * Runs in <head> of the landing page before the first paint, so a visitor who
 * chose the mode never sees the plain page flash. The class is on <html>,
 * outside the app, so hydration does not see it. It holds no `</`.
 */
export const prePaintScript = `(function () {
  try {
    var m = /${PARAM_RE.source}/i.exec(location.search);
    var on = m ? !/${OFF_VALUE_RE.source}/i.test(m[1] || "") : localStorage.getItem("${SHANTY_KEY}") === "1";
    if (on) document.documentElement.classList.add("${SHANTY_CLASS}");
  } catch (e) {}
})();`;
