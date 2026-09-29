// Sea shanty mode in the running site: the state the switches show, the flip
// between a page and its pirate variant, and the router hooks that show each
// page where the mode says it belongs as the visitor moves around. The rules
// are in shanty-mode.ts; this file only applies them.

import { inBrowser, type Router, useRoute } from "vitepress";
import { computed, nextTick, ref } from "vue";
import { data as pirate } from "../pirate.data";
import {
  announcement,
  browserStorage,
  type Choice,
  englishPath,
  isPiratePath,
  modeOn,
  paintShanty,
  paramState,
  readChoice,
  routeFor,
  SHANTY_KEY,
  stripShantyParam,
  themedPage,
  TURNING_CLASS,
  VISIT_KEY,
  visitFromAddress,
  writeChoice,
} from "./shanty-mode";

// The two choices (see shanty-mode.ts), read when the router is set up. The
// router hooks use these; what the switches show uses `mode`, which follows
// them only once the app is mounted, because the server rendered every page
// without them.
let visit: Choice = null;
let saved: Choice = null;
const on = () => modeOn(visit, saved);
const mode = ref(false);
// What a screen reader hears after a flip, and whether the ship is sailing.
const status = ref("");
const sailing = ref(false);
let statusTimer: ReturnType<typeof setTimeout> | undefined;
let shipTimer: ReturnType<typeof setTimeout> | undefined;
let router: Router | undefined;
// While set, navigation goes where it is sent: the switch itself is moving
// between a page and its variant.
let steering = false;
let busy = false;
// A page load the hooks turned away, whose after-hook must not repaint.
let redirected: string | undefined;

function setVisit(choice: Choice) {
  visit = choice;
  writeChoice(browserStorage("sessionStorage"), VISIT_KEY, choice);
}

/** Where `href` is shown instead, with its query (less `?shanty`) and fragment, or null. */
function elsewhere(href: string): string | null {
  const url = new URL(href, location.href);
  const to = routeFor(url.pathname, on(), pirate.missing);
  return to === null || to === url.pathname ? null : to + stripShantyParam(url.search) + url.hash;
}

/**
 * Hooks sea shanty mode into the router: a link with `?shanty` makes the
 * visit's choice, and every page, whether reached by a link or by Back and
 * Forward, is shown where the mode says it belongs.
 */
export function installShantyMode(app: { router: Router }): void {
  // The server renders each page where it is asked to.
  if (!inBrowser) return;
  router = app.router;
  saved = readChoice(browserStorage("localStorage"), SHANTY_KEY);
  // The pre-paint script has recorded this address's choice, unless
  // sessionStorage is blocked; then it is worked out again, for this page.
  let navigation = "navigate";
  try {
    navigation = (performance.getEntriesByType("navigation")[0] as PerformanceNavigationTiming).type;
  } catch {
    // Treated as a fresh visit.
  }
  visit = readChoice(browserStorage("sessionStorage"), VISIT_KEY) ?? visitFromAddress(location.pathname, location.search, navigation);

  const { onBeforeRouteChange, onBeforePageLoad, onAfterRouteChange } = router;
  router.onBeforeRouteChange = async (href) => {
    if ((await onBeforeRouteChange?.(href)) === false) return false;
    if (steering) return;
    const choice = paramState(new URL(href, location.href).search);
    if (choice !== null) setVisit(choice);
    const to = elsewhere(href);
    if (to === null) return;
    void router?.go(to);
    return false;
  };
  // The back and forward buttons load a page without asking the hook above.
  router.onBeforePageLoad = async (href) => {
    if ((await onBeforePageLoad?.(href)) === false) return false;
    const url = new URL(href, location.href);
    const shown = router?.route.path ?? "";
    // Moving between entries of the page on screen (its anchors) stays put.
    if (steering || url.pathname === shown) return;
    const choice = paramState(url.search);
    if (choice !== null) {
      setVisit(choice);
    } else if (englishPath(url.pathname) === englishPath(shown)) {
      // Back from a page to its own variant (after "Read the latest English
      // page", say) is the visitor choosing that variant again.
      setVisit(isPiratePath(url.pathname));
      return;
    }
    const to = elsewhere(href);
    if (to === null) return;
    const position = (history.state as { scrollPosition?: number } | null)?.scrollPosition;
    redirected = href;
    history.replaceState(history.state, "", to);
    void router?.go(to).then(() => {
      if (position) window.scrollTo(0, position);
    });
    return false;
  };
  router.onAfterRouteChange = async (href) => {
    await onAfterRouteChange?.(href);
    // The page on screen stays themed while the one it was sent to loads.
    if (href === redirected) {
      redirected = undefined;
      return;
    }
    paintShanty(document.documentElement, themedPage(location.pathname, on(), pirate.missing));
  };
}

/** Lets the switches show the choices, once the app is mounted; see `mode`. */
export function readStoredShanty(): void {
  mode.value = on();
  paintShanty(document.documentElement, themedPage(location.pathname, on(), pirate.missing));
}

/** Starts loading the mode's fonts, so a flip does not wait for them. */
export function warmShantyFonts(): void {
  try {
    for (const face of ['16px "Libre Caslon Text"', 'italic 16px "Libre Caslon Text"', '16px "IM Fell English"', '16px "IM Fell English SC"']) {
      void document.fonts.load(face);
    }
  } catch {
    // Older browsers load them when the text needs them.
  }
}

type Place = { id: string; offset: number };

/** The heading nearest above the top of the window, and how far below the nav it sits. */
function readingPlace(): Place | null {
  const nav = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--vp-nav-height")) || 64;
  let place: Place | null = null;
  for (const heading of document.querySelectorAll<HTMLElement>(".vp-doc :is(h2, h3, h4)[id]")) {
    const top = heading.getBoundingClientRect().top;
    if (top > nav + 8) break;
    place = { id: heading.id, offset: top };
  }
  return place;
}

/** The same heading in the other variant, put back where it was; returns the scroll position it set. */
function restorePlace(place: Place | null): number | null {
  const heading = place && document.getElementById(place.id);
  if (!place || !heading) return null;
  window.scrollTo(0, window.scrollY + heading.getBoundingClientRect().top - place.offset);
  return window.scrollY;
}

export function useShantyMode() {
  const route = useRoute();
  const page = computed(() => (route.path.startsWith("/") ? route.path : `/${route.path}`));
  /** Whether this page is in the mode: a pirate page, or an English page with no variant while the mode is on. */
  const shown = computed(() => themedPage(page.value, mode.value, pirate.missing));

  function announce(next: boolean, variant: boolean) {
    clearTimeout(statusTimer);
    status.value = announcement(next, variant);
    statusTimer = setTimeout(() => {
      status.value = "";
    }, 4000);
  }

  function sail(go: boolean) {
    clearTimeout(shipTimer);
    sailing.value = go;
    // The animation's end clears it; this is for a browser that never runs it.
    if (go) {
      shipTimer = setTimeout(() => {
        sailing.value = false;
      }, 4000);
    }
  }

  /**
   * Flips the mode and saves it: the page is replaced by its variant in
   * place, so the back button still goes to the page before, and the reader
   * stays at the same section. With view transitions the new page is
   * revealed by a circle growing from `origin`; visitors who ask for less
   * motion, and browsers without them, get the flip at once, with no ship.
   * Focus returns to the control matching `control` if it had it.
   */
  async function toggle(origin?: HTMLElement | null, control?: string): Promise<void> {
    if (busy) return;
    busy = true;
    const next = !shown.value;
    const focused = control ? document.activeElement?.closest(control) != null : false;
    saved = next;
    writeChoice(browserStorage("localStorage"), SHANTY_KEY, next);
    // The switch's choice replaces whatever an address chose for this visit.
    setVisit(null);
    mode.value = next;
    if (!next) sail(false);
    const here = location.pathname;
    const target = routeFor(here, next, pirate.missing);
    const search = stripShantyParam(location.search);
    const hash = location.hash;
    const place = readingPlace();
    let restored = null as number | null;

    const flip = async () => {
      if (target && router) {
        // Without its fragment, so VitePress does not scroll to it over the
        // reader's place; the fragment is put back once the page is shown.
        steering = true;
        try {
          history.replaceState(history.state, "", target + search);
          await router.go(target + search);
          history.replaceState(history.state, "", target + search + hash);
        } finally {
          steering = false;
        }
      } else if (search !== location.search) {
        // No variant to go to: the page stays and changes only its theme.
        history.replaceState(history.state, "", here + search + hash);
      }
      paintShanty(document.documentElement, themedPage(location.pathname, next, pirate.missing));
      await nextTick();
      restored = restorePlace(place);
    };

    const root = document.documentElement;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
    try {
      if (reduced || typeof document.startViewTransition !== "function") {
        await flip();
      } else {
        const box = origin?.getBoundingClientRect();
        const x = box ? box.left + box.width / 2 : innerWidth / 2;
        const y = box ? box.top + box.height / 2 : 0;
        const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y));
        let flipped: Promise<void> | undefined;
        root.classList.add(TURNING_CLASS);
        try {
          const transition = document.startViewTransition(() => (flipped ??= flip()));
          await transition.ready;
          root.animate(
            { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`] },
            { duration: 850, easing: "cubic-bezier(0.4, 0, 0.2, 1)", pseudoElement: "::view-transition-new(root)" },
          );
          await transition.finished;
        } catch {
          // A skipped transition (a hidden tab) still ran its callback; make sure.
          await (flipped ??= flip());
        } finally {
          root.classList.remove(TURNING_CLASS);
        }
        // The page is frozen while a transition runs, so the ship joins after it.
        if (next) sail(true);
      }
      if (focused && control) {
        // VitePress moves focus to the top of each new page; a switch keeps it.
        requestAnimationFrame(() => document.querySelector<HTMLElement>(control)?.focus({ preventScroll: true }));
      }
      // The mode's fonts can arrive after the flip and move the text; put the
      // reader back once they have, unless they have scrolled since. Putting
      // back a heading that has not moved changes nothing.
      if (restored !== null && document.fonts) {
        await Promise.race([document.fonts.ready, new Promise((done) => setTimeout(done, 1500))]);
        if (Math.abs(window.scrollY - restored) < 2) restorePlace(place);
      }
    } finally {
      busy = false;
    }
    announce(next, target !== null || !next);
  }

  /** The ship reached the far side. */
  function landed() {
    sailing.value = false;
  }

  return { on: shown, toggle, status, sailing, landed };
}
