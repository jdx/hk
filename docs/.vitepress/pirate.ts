// The pirate locale: sea shanty mode's words for VitePress's own interface,
// and the Markdown rules for pages under docs/pirate/. The pages themselves,
// and the checks that keep them in step with their English pages, are in
// pirate-pages.mjs; the switch and its rules are in theme/shanty-mode.ts.

import type { DefaultTheme, MarkdownEnv } from "vitepress";
import type MarkdownIt from "markdown-it";
import { posix } from "node:path";
import { englishPages, pirateVariants } from "./pirate-pages.mjs";
import { PIRATE_SIDEBAR_TEXT, sidebar, translateSidebar } from "./sidebar";
import { englishPath, fileKey, hasVariant, pageKey, PIRATE_LOCALE, PIRATE_PREFIX, piratePath } from "./theme/shanty-mode";

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
  const to = (link: string) => (moves(link) ? `/${PIRATE_LOCALE}${link}` : link);
  return {
    nav: [
      {
        text: "Charts",
        link: to("/getting_started"),
        activeMatch: "^/pirate/(getting_started|hooks|ci|mise_integration|logging)",
      },
      { text: "Rigging", link: to("/configuration") },
      { text: "Crew", link: to("/builtins") },
      { text: "Orders", link: to("/cli/"), activeMatch: "^/pirate/cli/" },
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

/** Variants whose English page is gone. They are left out of the build (`status` lists them), so a removed page never breaks it. */
export function orphanedVariants(): string[] {
  const english = new Set(englishPages());
  return pirateVariants().filter((page) => !english.has(page));
}

/**
 * In a pirate page, a link to another page goes to that page's pirate
 * variant. Variants are written with the English page's links (the checks in
 * pirate-pages.mjs compare them), so this resolves each one as the page is
 * rendered, before VitePress normalizes it and checks for dead links: to the
 * variant when there is one, else to the English page. A variant written from
 * an older English page can still link to a page that has since been
 * removed; that link becomes plain text, so a change to the English docs
 * never breaks the build through a stale variant. Links to files that are
 * not pages, and to other sites, are left alone.
 */
export function piratePlugin(md: MarkdownIt): void {
  const english = new Set(englishPages().map(fileKey));
  const variants = new Set(pirateVariants().filter((page) => english.has(fileKey(page))).map(fileKey));
  const unlinked = new WeakSet<object>();
  const linkOpen = md.renderer.rules.link_open!;
  const linkClose = md.renderer.rules.link_close ?? ((tokens, idx, options, _env, self) => self.renderToken(tokens, idx, options));
  md.renderer.rules.link_open = (tokens, idx, options, env: MarkdownEnv, self) => {
    const token = tokens[idx];
    const href = token.attrGet("href");
    if (isPiratePage(env) && href && !/^(?:[a-z][a-z0-9+.-]*:|\/\/|#)/i.test(href)) {
      const [, path, rest] = /^([^?#]*)(.*)$/.exec(href)!;
      const site = posix.resolve(posix.dirname(`/${env.relativePath}`), path) + (path.endsWith("/") && path !== "/" ? "/" : "");
      const page = englishPath(site);
      if (!/\.(?!md$|html$)[a-z0-9]+$/i.test(page)) {
        const key = pageKey(page.replace(/\.md$/, ""));
        if (variants.has(key)) {
          token.attrSet("href", piratePath(page) + rest);
        } else if (english.has(key)) {
          token.attrSet("href", page + rest);
        } else {
          // Find the matching close so it renders as the text's end, not a link's.
          let depth = 0;
          for (let j = idx + 1; j < tokens.length; j++) {
            if (tokens[j].type === "link_open") depth++;
            else if (tokens[j].type === "link_close" && depth-- === 0) {
              unlinked.add(tokens[j]);
              break;
            }
          }
          return "<span>";
        }
      }
    }
    return linkOpen(tokens, idx, options, env, self);
  };
  md.renderer.rules.link_close = (tokens, idx, options, env: MarkdownEnv, self) =>
    unlinked.has(tokens[idx]) ? "</span>" : linkClose(tokens, idx, options, env, self);

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
