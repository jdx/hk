import { defineConfig } from 'vitepress'

import spec from "../cli/commands.json";

function getCommands(cmd): string[][] {
  const commands = [];
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
  appearance: "dark",
  sitemap: {
    hostname: "https://hk.jdx.dev",
  },
  themeConfig: {
    // https://vitepress.dev/reference/default-theme-config
    nav: [
      { text: 'Getting Started', link: '/getting_started' },
      { text: 'Configuration', link: '/configuration' },
    ],
    sidebar: [
      { text: 'Getting Started', link: '/getting_started' },
      { text: 'Configuration', link: '/configuration' },
      { text: 'Environment Variables', link: '/environment_variables' },
      { text: 'About', link: '/about' },
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
    head: [
      [
        "script",
        {
          async: "",
          src: "https://www.googletagmanager.com/gtag/js?id=G-B69G389C8T",
        },
      ],
      [
        "script",
        {},
        `window.dataLayer = window.dataLayer || [];
        function gtag(){dataLayer.push(arguments);}
        gtag('js', new Date());
        gtag('config', 'G-B69G389C8T');`,
      ],
      [
        "script",
        {
          "data-goatcounter": "https://jdx.goatcounter.com/count",
          async: "",
          src: "//gc.zgo.at/count.js",
        },
      ],
    ],
  }
})
