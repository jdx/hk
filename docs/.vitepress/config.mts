import { socialCard, writeSocialCard } from "./social-images.mjs";
import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitepress'

import pklLang from '../pkl.tmLanguage.json'
import { sidebar } from './sidebar'
const configDir = dirname(fileURLToPath(import.meta.url));
const cargoToml = readFileSync(resolve(configDir, '../../Cargo.toml'), 'utf8');
const versionMatch = cargoToml.match(/^\[package\][\s\S]*?^\s*version\s*=\s*"([^"]+)"/m);
if (!versionMatch) {
  console.warn('Unable to find package version in Cargo.toml');
}
const latestVersion = versionMatch?.[1] ?? '0.0.0';
const siteUrl = "https://hk.jdx.dev";
const siteDescription =
  "Fast, language-agnostic git hooks and project linting with parallel execution, automatic fixes, file locking, and shareable Pkl configuration.";

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "hk",
  description: siteDescription,
  lang: "en-US",
  lastUpdated: true,
  appearance: "force-dark",
  sitemap: {
    hostname: siteUrl,
  },
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    logo: '/logo-small.png',
    nav: [
      { text: 'Getting Started', link: '/getting_started' },
      { text: 'Contributing', link: '/contributing' },
      { text: 'Configuration', link: '/configuration' },
      { text: 'CLI Reference', link: '/cli/' },
      { text: `v${latestVersion}`, link: 'https://github.com/jdx/hk/releases' },
    ],
    sidebar,
    socialLinks: [
      { icon: 'github', link: 'https://github.com/jdx/hk' },
      { icon: 'discord', link: 'https://discord.gg/UBa7pJUN7Z' },
    ],
    editLink: {
      pattern: "https://github.com/jdx/hk/edit/main/docs/:path",
    },
    search: {
      provider: 'local',
    },
    footer: false,
  },
  markdown: {
    // https://github.com/vuejs/vitepress/discussions/3724
    config(md) {
      const defaultCodeInline = md.renderer.rules.code_inline!
      md.renderer.rules.code_inline = (tokens, idx, options, env, self) => {
        tokens[idx].attrSet('v-pre', '')
        return defaultCodeInline(tokens, idx, options, env, self)
      }
    },
    languages: [{
      name: 'pkl',
      displayName: 'pkl',
      scopeName: 'source.pkl',
      repository: {},
      patterns: pklLang.patterns as any,
    }]
  },
  head: [
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
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:locale", content: "en_US" }],
    ["meta", { property: "og:image:width", content: "1200" }],
    ["meta", { property: "og:image:height", content: "630" }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:site", content: "@jdxcode" }],
    ["link", { rel: "icon", href: "/favicon.ico", sizes: "any" }],
    ["link", { rel: "icon", type: "image/png", sizes: "32x32", href: "/favicon-32x32.png" }],
    ["link", { rel: "apple-touch-icon", sizes: "180x180", href: "/apple-touch-icon.png" }],
    ["link", { rel: "manifest", href: "/site.webmanifest" }],
    ["meta", { name: "theme-color", content: "#0d0221" }],
  ],
  transformHead({ pageData, title, description, siteConfig }) {
    const heading =
      pageData.relativePath === "index.md"
        ? "Fast git hooks and project linting"
        : pageData.title || "hk";
    const card = socialCard(heading);
    writeSocialCard(siteConfig.outDir, card);
    const image = new URL(card.path, `${siteUrl}/`).toString();
    const imageAlt = `${heading} — hk docs`;
    const url = `${siteUrl}/${pageData.relativePath}`
      .replace(/index\.md$/, "")
      .replace(/\.md$/, ".html");

    return [
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
})
