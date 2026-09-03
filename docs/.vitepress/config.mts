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

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "hk",
  description: "git hook manager",
  lang: "en-US",
  lastUpdated: true,
  appearance: "force-dark",
  sitemap: {
    hostname: "https://hk.jdx.dev",
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
    ["meta", { property: "og:image", content: "https://hk.jdx.dev/og.png" }],
    ["meta", { property: "og:image:width", content: "1200" }],
    ["meta", { property: "og:image:height", content: "630" }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:image", content: "https://hk.jdx.dev/og.png" }],
  ],
})
