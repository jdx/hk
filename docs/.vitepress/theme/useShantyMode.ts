// Sea shanty mode in the running site: the state the switches show, the flip
// between a page and its pirate variant, and the router hooks that keep a
// visitor in the mode as they move around. The rules are in shanty-mode.ts;
// this file only applies them.

import { inBrowser, type Router, useRoute } from "vitepress";
import { computed, nextTick, ref } from "vue";
import { data as pirate } from "../pirate.data";
import {
  announcement,
  browserStorage,
  englishPath,
  hasVariant,
  isPiratePath,
  paintShanty,
  piratePath,
  rememberShanty,
  storedShanty,
  stripShantyParam,
  themedAfterNavigation,
  TURNING_CLASS,
} from "./shanty-mode";

// The visitor's saved choice. It is read once the app is mounted, never while
// hydrating: the server rendered every page without it.
const stored = ref(false);
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

const hrefPath = (href: string) => new URL(href, location.href).pathname;
const hrefRest = (href: string) => {
  const url = new URL(href, location.href);
  return stripShantyParam(url.search) + url.hash;
};

/**
 * Hooks sea shanty mode into the router. From a pirate page, or with the mode
 * saved on, a link to an English page that has a variant goes to the variant,
 * and so does the browser's back button with the mode saved on; an English
 * page with no variant is themed in place.
 */
export function installShantyMode(app: { router: Router }): void {
  // The server renders each page where it is asked to.
  if (!inBrowser) return;
  router = app.router;
  const { onBeforeRouteChange, onBeforePageLoad, onAfterRouteChange } = router;
  const toVariant = (href: string) => {
    const path = hrefPath(href);
    return !steering && !isPiratePath(path) && hasVariant(path, pirate.missing) ? piratePath(path) + hrefRest(href) : null;
  };
  router.onBeforeRouteChange = async (href) => {
    if ((await onBeforeRouteChange?.(href)) === false) return false;
    const variant = (isPiratePath(location.pathname) || stored.value) && toVariant(href);
    if (!variant) return;
    void router?.go(variant);
    return false;
  };
  // The back and forward buttons load a page without asking the hook above.
  router.onBeforePageLoad = async (href) => {
    if ((await onBeforePageLoad?.(href)) === false) return false;
    const variant = stored.value && toVariant(href);
    if (!variant) return;
    history.replaceState(history.state, "", variant);
    void router?.go(variant);
    return false;
  };
  router.onAfterRouteChange = async (href) => {
    await onAfterRouteChange?.(href);
    paintShanty(document.documentElement, themedAfterNavigation(hrefPath(href), stored.value, pirate.missing));
  };
}

/** Reads the saved choice once the app is mounted; see `stored`. */
export function readStoredShanty(): void {
  stored.value = storedShanty(browserStorage());
  paintShanty(document.documentElement, themedAfterNavigation(location.pathname, stored.value, pirate.missing));
}

/** The heading nearest above the top of the window, and how far below the nav it sits. */
function readingPlace(): { id: string; offset: number } | null {
  const nav = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--vp-nav-height")) || 64;
  let place: { id: string; offset: number } | null = null;
  for (const heading of document.querySelectorAll<HTMLElement>(".vp-doc :is(h2, h3, h4)[id]")) {
    const top = heading.getBoundingClientRect().top;
    if (top > nav + 8) break;
    place = { id: heading.id, offset: top };
  }
  return place;
}

/** The same heading in the other variant, put back where it was. */
function restorePlace(place: { id: string; offset: number } | null): void {
  if (!place) return;
  const heading = document.getElementById(place.id);
  if (heading) window.scrollTo(0, window.scrollY + heading.getBoundingClientRect().top - place.offset);
}

export function useShantyMode() {
  const route = useRoute();
  const page = computed(() => (route.path.startsWith("/") ? route.path : `/${route.path}`));
  /** Whether the mode is on for this page: a pirate page, or an English page with no variant while it is saved on. */
  const on = computed(() => themedAfterNavigation(page.value, stored.value, pirate.missing));

  function announce(next: boolean, variant: boolean) {
    clearTimeout(statusTimer);
    status.value = announcement(next, variant);
    statusTimer = setTimeout(() => {
      status.value = "";
    }, 4000);
  }

  function sail() {
    sailing.value = true;
    clearTimeout(shipTimer);
    // The animation's end clears it; this is for a browser that never runs it.
    shipTimer = setTimeout(() => {
      sailing.value = false;
    }, 4000);
  }

  /**
   * Flips the mode and remembers it: the page is replaced by its variant in
   * place, so the back button still goes to the page before, and the reader
   * stays at the same section. With view transitions the new page is
   * revealed by a circle growing from `origin`; visitors who ask for less
   * motion, and browsers without them, get the flip at once, with no ship.
   */
  async function toggle(origin?: HTMLElement | null): Promise<void> {
    if (busy) return;
    busy = true;
    const next = !on.value;
    const here = location.pathname;
    const target = next ? (hasVariant(here, pirate.missing) ? piratePath(here) : null) : isPiratePath(here) ? englishPath(here) : null;
    const place = readingPlace();
    rememberShanty(next, browserStorage());
    stored.value = next;

    const flip = async () => {
      if (target && router) {
        const href = target + stripShantyParam(location.search) + location.hash;
        steering = true;
        try {
          history.replaceState(history.state, "", href);
          await router.go(href);
        } finally {
          steering = false;
        }
      } else {
        // No variant to go to: the page stays and changes only its theme.
        const search = stripShantyParam(location.search);
        if (search !== location.search) history.replaceState(history.state, "", location.pathname + search + location.hash);
      }
      paintShanty(document.documentElement, themedAfterNavigation(location.pathname, next, pirate.missing));
      await nextTick();
      restorePlace(place);
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
        if (next) sail();
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

  /** Reads this page in English without leaving the mode: the next link goes back to pirate. */
  async function readEnglish(): Promise<void> {
    if (!router) return;
    steering = true;
    try {
      await router.go(englishPath(location.pathname) + stripShantyParam(location.search) + location.hash);
    } finally {
      steering = false;
    }
  }

  return { on, toggle, status, sailing, landed, readEnglish, stored };
}
