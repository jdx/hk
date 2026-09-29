import { socialCard, writeSocialCard } from "./social-images.mjs";
import { shantyFiles } from "./shanty.data";
import { showreelFiles } from "./showreel.data";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig, type HeadConfig } from "vitepress";

import pklLang from "../pkl.tmLanguage.json";
import { isCurrent, PIRATE_DIR } from "./pirate-pages.mjs";
import { missingVariants, piratePlugin, pirateSearch, pirateThemeConfig } from "./pirate";
import { sidebar } from "./sidebar";
import { prePaintScript as shantyModeScript, SHANTY_CLASS } from "./theme/shanty-mode";
const configDir = dirname(fileURLToPath(import.meta.url));
const cargoToml = readFileSync(resolve(configDir, "../../Cargo.toml"), "utf8");
const versionMatch = cargoToml.match(
  /^\[package\][\s\S]*?^\s*version\s*=\s*"([^"]+)"/m,
);
if (!versionMatch) {
  console.warn("Unable to find package version in Cargo.toml");
}
const latestVersion = versionMatch?.[1] ?? "0.0.0";
const siteUrl = "https://hk.jdx.dev";
const siteDescription =
  "Fast, language-agnostic git hooks and project linting with parallel execution, automatic fixes, file locking, and shareable Pkl configuration.";

// Link previews that play video (Discord, iMessage, Telegram) use a page's
// rendered video through og:video: the showreel on the homepage, the music
// video on the shanty's page. X ignores og:video and keeps the large image
// card. Builds without a render leave the tags out. The pirate landing page
// is sea shanty mode's, so it shares the music video.
function videoTags(src: string | undefined): [string, Record<string, string>][] {
  if (!src) return [];
  const url = `${siteUrl}${src}`;
  return [
    ["meta", { property: "og:video", content: url }],
    ["meta", { property: "og:video:secure_url", content: url }],
    ["meta", { property: "og:video:type", content: "video/mp4" }],
    ["meta", { property: "og:video:width", content: "1920" }],
    ["meta", { property: "og:video:height", content: "1080" }],
  ];
}

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "hk",
  description: siteDescription,
  lang: "en-US",
  lastUpdated: true,
  appearance: "dark",
  // Included reference fragments are not standalone pages, and the pirate
  // pages' style guide is for their writers.
  srcExclude: ["gen/**", `${PIRATE_DIR}/STYLE.md`],
  sitemap: {
    hostname: siteUrl,
    // Search engines get the English pages; the pirate ones are for fun.
    transformItems: (items) => items.filter((item) => !item.url.startsWith(`${PIRATE_DIR}/`)),
  },
  // Sea shanty mode (theme/shanty-mode.ts) is a second locale: every English
  // page has a pirate variant under /pirate/, with the interface's words from
  // pirate.ts. Neither locale has a label, so VitePress shows no language
  // menu; the header's switch moves between them.
  locales: {
    root: { label: "", lang: "en-US" },
    [PIRATE_DIR]: {
      label: "",
      lang: "en-x-pirate",
      link: `/${PIRATE_DIR}/`,
      description:
        "Git hooks fer linters and formatters, sung by the crew: hk runs yer steps in parallel, lashes shared files so no two hands collide, and stows unstaged work safe in the hold.",
      themeConfig: pirateThemeConfig(latestVersion),
    },
  },
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    logo: "/logo-small.png",
    nav: [
      {
        text: "Guide",
        link: "/getting_started",
        activeMatch: "^/(getting_started|hooks|ci|mise_integration|logging)",
      },
      { text: "Configuration", link: "/configuration" },
      { text: "Builtins", link: "/builtins" },
      { text: "CLI", link: "/cli/", activeMatch: "^/cli/" },
      { text: `v${latestVersion}`, link: "https://github.com/jdx/hk/releases" },
    ],
    sidebar,
    socialLinks: [
      { icon: "github", link: "https://github.com/jdx/hk" },
      { icon: "discord", link: "https://discord.gg/UBa7pJUN7Z" },
    ],
    editLink: {
      pattern: "https://github.com/jdx/hk/edit/main/docs/:path",
    },
    search: {
      provider: "local",
      options: { locales: { [PIRATE_DIR]: pirateSearch } },
    },
    outline: { level: [2, 3], label: "On this page" },
  },
  markdown: {
    // https://github.com/vuejs/vitepress/discussions/3724
    config(md) {
      const defaultCodeInline = md.renderer.rules.code_inline!;
      md.renderer.rules.code_inline = (tokens, idx, options, env, self) => {
        tokens[idx].attrSet("v-pre", "");
        return defaultCodeInline(tokens, idx, options, env, self);
      };
      piratePlugin(md);
    },
    languages: [
      {
        name: "pkl",
        displayName: "pkl",
        scopeName: "source.pkl",
        repository: {},
        patterns: pklLang.patterns as any,
      },
    ],
  },
  head: [
    // Before anything paints: a visitor who chose sea shanty mode goes to the
    // pirate variant of the page they opened, and a themed page never
    // flashes plain.
    ["script", {}, shantyModeScript(missingVariants())],
    [
      "script",
      {},
      `(function () {
  try {
    var d = document.documentElement;
    var c = JSON.parse(localStorage.getItem("jdx-banner-cache") || "null");
    var expires = c && c.expires ? Date.parse(c.expires) : NaN;
    var now = Date.now();
    var metadataValid =
      c &&
      typeof c.id === "string" &&
      typeof c.height === "string" &&
      /^[1-9]\\d*(?:\\.\\d+)?px$/.test(c.height) &&
      Number.isFinite(c.width) &&
      typeof c.fontSize === "string" &&
      Number.isFinite(c.pixelRatio) &&
      Number.isFinite(c.cachedAt) &&
      c.cachedAt <= now &&
      now - c.cachedAt < 300000 &&
      (!c.expires || (typeof c.expires === "string" && Number.isFinite(expires) && now < expires));
    var contextMatches =
      metadataValid &&
      c.width === innerWidth &&
      c.fontSize === getComputedStyle(d).fontSize &&
      c.pixelRatio === devicePixelRatio;
    if (contextMatches && localStorage.getItem("jdx-banner-dismissed") !== c.id)
      d.style.setProperty("--vp-layout-top-height", c.height);
    else if (c && !metadataValid)
      localStorage.removeItem("jdx-banner-cache");
  } catch (e) {}
})();`,
    ],
    // OpenGraph
    ["meta", { property: "og:site_name", content: "hk" }],
    // og:type is set per page in transformHead: with a rendered video, the
    // homepage and the shanty's page are a video.other.
    ["meta", { property: "og:locale", content: "en_US" }],
    ["meta", { property: "og:image:width", content: "1200" }],
    ["meta", { property: "og:image:height", content: "630" }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:site", content: "@jdxcode" }],
    ["link", { rel: "icon", href: "/favicon.ico", sizes: "16x16 32x32 48x48" }],
    [
      "link",
      {
        rel: "icon",
        type: "image/png",
        sizes: "32x32",
        href: "/favicon-32x32.png",
      },
    ],
    // Last, and the only scalable one, so browsers that read SVG favicons
    // pick it: it follows the browser's light or dark theme.
    ["link", { rel: "icon", type: "image/svg+xml", href: "/favicon.svg" }],
    [
      "link",
      {
        rel: "apple-touch-icon",
        sizes: "180x180",
        href: "/apple-touch-icon.png",
      },
    ],
    ["link", { rel: "manifest", href: "/site.webmanifest" }],
    ["meta", { name: "theme-color", content: "#101a23" }],
  ],
  transformPageData(pageData) {
    // A pirate page written from an older English page says so, and links to
    // the English one (theme/ShantyNotice.vue).
    const prefix = `${PIRATE_DIR}/`;
    if (pageData.relativePath.startsWith(prefix)) {
      pageData.frontmatter.pirateStale = !isCurrent(pageData.relativePath.slice(prefix.length));
    }
  },
  transformHtml(html, _id, { pageData }) {
    // Pirate pages are themed from the first byte, with or without JavaScript.
    if (!pageData.relativePath.startsWith(`${PIRATE_DIR}/`)) return html;
    return html.replace(/<html(?![^>]*\bclass=)/, `<html class="${SHANTY_CLASS}"`);
  },
  transformHead({ pageData, title, description, siteConfig }) {
    const pirate = pageData.relativePath.startsWith(`${PIRATE_DIR}/`);
    const heading =
      pageData.relativePath === "index.md"
        ? "Fast git hooks and project linting"
        : pageData.relativePath === `${PIRATE_DIR}/index.md`
          ? "Git hooks, sung by the crew"
          : pageData.title || "hk";
    const card = socialCard(heading);
    writeSocialCard(siteConfig.outDir, card);
    const image = new URL(card.path, `${siteUrl}/`).toString();
    const imageAlt = `${heading} — hk docs`;
    const url = `${siteUrl}/${pageData.relativePath}`
      .replace(/index\.md$/, "")
      .replace(/\.md$/, ".html");
    const video = videoTags(
      pageData.relativePath === "index.md"
        ? showreelFiles()?.src
        : ["shanty.md", `${PIRATE_DIR}/index.md`, `${PIRATE_DIR}/shanty.md`].includes(pageData.relativePath)
          ? shantyFiles().video?.src
          : undefined,
    );

    // The pirate pages repeat the English ones in other words; search
    // engines index the English pages only.
    const robots: HeadConfig[] = pirate ? [["meta", { name: "robots", content: "noindex, follow" }]] : [];

    return [
      ...robots,
      [
        "meta",
        {
          property: "og:type",
          content: video.length ? "video.other" : "website",
        },
      ],
      ...video,
      ["link", { rel: "canonical", href: url }],
      ["meta", { property: "og:url", content: url }],
      ["meta", { property: "og:image", content: image }],
      ["meta", { property: "og:image:alt", content: imageAlt }],
      ["meta", { name: "twitter:image", content: image }],
      ["meta", { name: "twitter:image:alt", content: imageAlt }],
      ["meta", { property: "og:title", content: title }],
      ["meta", { property: "og:description", content: description }],
      ["meta", { name: "twitter:title", content: title }],
      ["meta", { name: "twitter:description", content: description }],
      [
        "script",
        { type: "application/ld+json" },
        JSON.stringify({
          "@context": "https://schema.org",
          "@type": "WebPage",
          name: title,
          description,
          url,
          isPartOf: { "@type": "WebSite", name: "hk", url: siteUrl },
        }),
      ],
    ];
  },
});
