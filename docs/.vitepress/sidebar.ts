import spec from "../cli/commands.json" with { type: "json" };
import type { DefaultTheme } from "vitepress";
export type SidebarItem = DefaultTheme.SidebarItem;

interface Command {
  subcommands?: Record<string, Command>;
  hide?: boolean;
  full_cmd?: string[];
}

function commandItems(cmd: Command): DefaultTheme.SidebarItem[] {
  return Object.entries(cmd.subcommands ?? {}).flatMap(([name, sub]) => {
    const items = commandItems(sub);
    if (sub.hide) return items;
    return [
      {
        text: sub.full_cmd?.join(" ") ?? name,
        link: `/cli/${sub.full_cmd?.join("/") ?? name}`,
        ...(items.length ? { collapsed: true, items } : {}),
      },
    ];
  });
}

export const sidebar: SidebarItem[] = [
  {
    text: "Start here",
    items: [
      { text: "Getting started", link: "/getting_started" },
      { text: "Migrating to hk v2", link: "/migration-v2" },
      { text: "Why hk?", link: "/why-hk" },
      { text: "Pkl essentials", link: "/pkl_introduction" },
    ],
  },
  {
    text: "Guides",
    items: [
      { text: "Git hooks and stashing", link: "/hooks" },
      { text: "Continuous integration", link: "/ci" },
      { text: "mise integration", link: "/mise_integration" },
      { text: "Troubleshooting", link: "/logging" },
      { text: "Coding agents", link: "/agents" },
      {
        text: "Configuration examples",
        link: "/reference/examples/",
        collapsed: false,
        items: [
          {
            text: "JavaScript and TypeScript",
            link: "/reference/examples/javascript-project",
          },
          { text: "Python", link: "/reference/examples/python-project" },
          { text: "Monorepo", link: "/reference/examples/monorepo" },
          {
            text: "Custom steps",
            link: "/reference/examples/custom-linters",
          },
        ],
      },
    ],
  },
  {
    text: "Reference",
    items: [
      { text: "Configuration", link: "/configuration" },
      { text: "Built-in linters", link: "/builtins" },
      { text: "Environment variables", link: "/environment_variables" },
      { text: "Glossary", link: "/glossary" },
      {
        text: "CLI commands",
        link: "/cli/",
        collapsed: true,
        items: commandItems(spec.cmd),
      },
    ],
  },
  {
    text: "Project",
    items: [
      { text: "Benchmarks", link: "/benchmarks" },
      { text: "About hk", link: "/about" },
      { text: "Contributing", link: "/contributing" },
      { text: "Sea shanty", link: "/shanty" },
    ],
  },
];

/**
 * The sidebar's words in sea shanty mode, keyed by the English words. Each
 * keeps the English subject in view, so a reader can still find a page by
 * what it is about. Commands keep their names.
 */
export const PIRATE_SIDEBAR_TEXT: Record<string, string> = {
  "Start here": "Weigh anchor",
  "Getting started": "Getting underway",
  "Migrating to hk v2": "Refitting for hk v2",
  "Why hk?": "Why sail with hk?",
  "Pkl essentials": "Pkl for landlubbers",
  Guides: "Sailing orders",
  "Git hooks and stashing": "Hooks and stowing the hold",
  "Continuous integration": "The harbour-master (CI)",
  "mise integration": "Provisioning with mise",
  Troubleshooting: "Foul weather (troubleshooting)",
  "Coding agents": "Clockwork crew (coding agents)",
  "Configuration examples": "Ships in bottles (examples)",
  "JavaScript and TypeScript": "A JavaScript and TypeScript sloop",
  Python: "A Python brig",
  Monorepo: "A whole fleet (monorepo)",
  "Custom steps": "Custom-rigged steps",
  Reference: "Charts and tables",
  Configuration: "Ship's articles (configuration)",
  "Built-in linters": "The standing crew (builtins)",
  "Environment variables": "Winds and currents (environment)",
  Glossary: "Sailor's lexicon (glossary)",
  "CLI commands": "Bosun's calls (CLI)",
  Project: "The ship's company",
  Benchmarks: "Speed trials (benchmarks)",
  "About hk": "About the vessel",
  Contributing: "Signing aboard (contributing)",
  "Sea shanty": "The shanty",
};

/**
 * A sidebar with its words from `text` and its links moved under `prefix`,
 * except the links `moves` turns down, which keep their English page.
 */
export function translateSidebar(
  items: SidebarItem[],
  prefix: string,
  text: Record<string, string>,
  moves: (link: string) => boolean = () => true,
): SidebarItem[] {
  return items.map((item) => ({
    ...item,
    text: item.text && (text[item.text] ?? item.text),
    ...(item.link && moves(item.link) ? { link: `${prefix}${item.link}` } : {}),
    ...(item.items ? { items: translateSidebar(item.items, prefix, text, moves) } : {}),
  }));
}
