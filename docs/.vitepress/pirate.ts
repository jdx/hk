// The pirate locale: sea shanty mode's words for VitePress's own interface,
// and the Markdown rules for pages under docs/pirate/. The pages themselves,
// and the checks that keep them in step with their English pages, are in
// pirate-pages.mjs; the switch and its rules are in theme/shanty-mode.ts.

import type { DefaultTheme, MarkdownEnv } from "vitepress";
import type MarkdownIt from "markdown-it";
import { englishPages, pirateVariants } from "./pirate-pages.mjs";
import { PIRATE_SIDEBAR_TEXT, sidebar, translateSidebar } from "./sidebar";
import { fileKey, hasVariant, pageKey, PIRATE_LOCALE, PIRATE_PREFIX } from "./theme/shanty-mode";

/** English pages with no pirate variant, as `fileKey`s; normally none. */
export function missingVariants(): string[] {
  const variants = new Set(pirateVariants());
  return englishPages()
    .filter((page) => !variants.has(page))
    .map(fileKey);
}

export function pirateThemeConfig(version: string): DefaultTheme.Config {
  const missing = missingVariants();
  // A page with no variant keeps its English link.
  const moves = (link: string) => hasVariant(link, missing);
  return {
    nav: [
      {
        text: "Charts",
        link: "/pirate/getting_started",
        activeMatch: "^/pirate/(getting_started|hooks|ci|mise_integration|logging)",
      },
      { text: "Rigging", link: "/pirate/configuration" },
      { text: "Crew", link: "/pirate/builtins" },
      { text: "Orders", link: "/pirate/cli/", activeMatch: "^/pirate/cli/" },
      { text: `v${version}`, link: "https://github.com/jdx/hk/releases" },
    ],
    sidebar: translateSidebar(sidebar, `/${PIRATE_LOCALE}`, PIRATE_SIDEBAR_TEXT, moves),
    outline: { level: [2, 3], label: "Yer bearings" },
    editLink: {
      pattern: "https://github.com/jdx/hk/edit/main/docs/:path",
      text: "Mend this chart on GitHub",
    },
    lastUpdated: { text: "Last charted" },
    docFooter: { prev: "Astern", next: "Ahead" },
    darkModeSwitchLabel: "Night watch",
    lightModeSwitchTitle: "Hoist the day lamps",
    darkModeSwitchTitle: "Douse the lamps for the night watch",
    sidebarMenuLabel: "Charts",
    returnToTopLabel: "Back to the crow's nest",
    skipToContentLabel: "Skip to the cargo",
    externalLinkIcon: false,
    notFound: {
      title: "LOST AT SEA",
      quote:
        "Ye've sailed clean off the edge o' the chart. There be naught here but fog and sea serpents.",
      linkLabel: "Back to port",
      linkText: "Back to port",
      code: "404",
    },
  };
}

/** The local search box's words in sea shanty mode. */
export const pirateSearch = {
  translations: {
    button: { buttonText: "Search the seas", buttonAriaLabel: "Search the seas" },
    modal: {
      displayDetails: "Unroll the full chart",
      resetButtonTitle: "Clear the search",
      backButtonTitle: "Abandon the search",
      noResultsText: "Nary a trace of",
      footer: {
        selectText: "to board",
        selectKeyAriaLabel: "enter",
        navigateText: "to steer",
        navigateUpKeyAriaLabel: "up arrow",
        navigateDownKeyAriaLabel: "down arrow",
        closeText: "to abandon ship",
        closeKeyAriaLabel: "escape",
      },
    },
  },
};

/** Container titles in sea shanty mode, for containers that give none of their own. */
const CONTAINER_TITLES: Record<string, [string, string]> = {
  tip: ["TIP", "A WORD FROM THE PARROT"],
  info: ["INFO", "FROM THE SHIP'S LOG"],
  warning: ["WARNING", "BEWARE"],
  danger: ["DANGER", "HERE BE DRAGONS"],
  details: ["Details", "Open the sea chest"],
};

const isPiratePage = (env: MarkdownEnv) => env.relativePath?.startsWith(PIRATE_PREFIX.slice(1)) === true;

/**
 * In a pirate page, a link to another page goes to that page's pirate
 * variant. Variants are written with the English page's links (the checks in
 * pirate-pages.mjs compare them), so this moves them under /pirate/ as the
 * page is rendered, before VitePress normalizes them and checks for dead
 * links. A page with no variant, and any file that is not a page, keeps its
 * English link.
 */
export function piratePlugin(md: MarkdownIt): void {
  const variants = new Set(pirateVariants().map(fileKey));
  const linkOpen = md.renderer.rules.link_open!;
  md.renderer.rules.link_open = (tokens, idx, options, env: MarkdownEnv, self) => {
    const href = tokens[idx].attrGet("href");
    if (isPiratePage(env) && href?.startsWith("/") && !href.startsWith("//") && !href.startsWith(PIRATE_PREFIX)) {
      const [, path, rest] = /^([^?#]*)(.*)$/.exec(href)!;
      if (variants.has(pageKey(path.replace(/\.md$/, "")))) tokens[idx].attrSet("href", `/${PIRATE_LOCALE}${path}${rest}`);
    }
    return linkOpen(tokens, idx, options, env, self);
  };

  for (const [name, [english, pirate]] of Object.entries(CONTAINER_TITLES)) {
    const rule = `container_${name}_open`;
    const open = md.renderer.rules[rule];
    if (!open) continue;
    md.renderer.rules[rule] = (tokens, idx, options, env: MarkdownEnv, self) => {
      const html = open(tokens, idx, options, env, self);
      const own = tokens[idx].info.trim().slice(name.length).trim();
      if (!isPiratePage(env) || own) return html;
      return name === "details"
        ? html.replace(`<summary>${english}</summary>`, `<summary>${pirate}</summary>`)
        : html.replace(`<p class="custom-block-title">${english}</p>`, `<p class="custom-block-title">${pirate}</p>`);
    };
  }
}
