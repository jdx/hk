import { defineConfig } from 'vitepress'

import spec from "../cli/commands.json";
import pklLang from '../pkl.tmLanguage.json'

interface Command {
  subcommands: Record<string, Command & { hide?: boolean; full_cmd: string[] }>;
}

function getCommands(cmd: Command): string[][] {
  const commands: string[][] = [];
  for (const [name, sub] of Object.entries(cmd.subcommands)) {
    if (sub.hide) continue;
    commands.push(sub.full_cmd);
    commands.push(...getCommands(sub));
  }
  return commands;
}

const commands = getCommands(spec.cmd);

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
      { text: 'Configuration', link: '/configuration' },
      { text: 'CLI Reference', link: '/cli/' },
    ],
    sidebar: [
      { text: 'About', link: '/about' },
      { text: 'Sea Shanty', link: '/shanty' },
      { text: 'Getting Started', link: '/getting_started' },
      { text: 'Configuration', link: '/configuration' },
      {
        text: 'Reference',
        items: [
          { text: 'Built-in Linters', link: '/builtins' },
          { text: 'Configuration Examples', link: '/reference/examples/' },
          { text: 'Glossary', link: '/glossary' },
        ]
      },
      { text: 'Environment Variables', link: '/environment_variables' },
      { text: 'Hooks', link: '/hooks' },
      { text: 'Logging and Debugging', link: '/logging' },
      { text: 'Introduction to pkl', link: '/pkl_introduction' },
      { text: 'mise-en-place Integration', link: '/mise_integration' },
      { text: 'CLI Reference', link: '/cli', items: commands.map(cmd => ({ text: cmd.join(' '), link: `/cli/${cmd.join('/')}` })) },
    ],
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
    footer: {
      message:
        'Licensed under the MIT License. Maintained by <a href="https://github.com/jdx">@jdx</a> and <a href="https://github.com/jdx/hk/graphs/contributors">friends</a>.',
      copyright: `Copyright © ${new Date().getFullYear()} <a href="https://github.com/jdx">@jdx</a>`,
    },
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
      {
        async: "",
        src: "https://www.googletagmanager.com/gtag/js?id=G-M7TEP8PKSE",
      },
    ],
    [
      "script",
      {},
      `window.dataLayer = window.dataLayer || [];
      function gtag(){dataLayer.push(arguments);}
      gtag('js', new Date());
      gtag('config', 'G-M7TEP8PKSE');`,
    ],
    [
      "script",
      {
        "data-goatcounter": "https://jdx-hk.goatcounter.com/count",
        async: "",
        src: "//gc.zgo.at/count.js",
      },
    ],
    // OpenGraph
    ["meta", { property: "og:site_name", content: "hk" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:image", content: "https://hk.jdx.dev/android-chrome-512x512.png" }],
    ["meta", { name: "twitter:card", content: "summary" }],
    ["meta", { name: "twitter:image", content: "https://hk.jdx.dev/android-chrome-512x512.png" }],
  ],
})
