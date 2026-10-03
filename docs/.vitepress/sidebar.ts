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
      { text: "Migrating from lefthook", link: "/migrate-lefthook" },
      { text: "Migrating from husky", link: "/migrate-husky" },
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
      { text: "Security model", link: "/security" },
      { text: "Contributing", link: "/contributing" },
      { text: "Sea shanty", link: "/shanty" },
    ],
  },
];

/**
 * The sidebar's words in sea shanty mode, keyed by the English words. A page's
 * entry is its pirate `#` heading, or, when that heading is long, the
 * heading's own crew phrase with the English subject in brackets, so a reader
 * can still find a page by what it is about. Change an entry and the page's
 * heading together (docs/pirate/STYLE.md). Commands keep their names.
 */
export const PIRATE_SIDEBAR_TEXT: Record<string, string> = {
  "Start here": "Weigh anchor",
  "Getting started": "Getting under way",
  "Migrating to hk v2": "Refitting the ship for hk v2",
  "Migrating from lefthook": "Changing ships from lefthook",
  "Migrating from husky": "Changing ships from husky and lint-staged",
  "Why hk?": "Why sail with hk?",
  "Pkl essentials": "Pkl essentials for the chart room",
  Guides: "Seamanship",
  "Git hooks and stashing": "Git hooks and stowing the hold",
  "Continuous integration": "The harbour-master (CI)",
  "mise integration": "The quartermaster (mise)",
  Troubleshooting: "Foul weather (troubleshooting)",
  "Coding agents": "Clockwork hands: coding agents",
  "Configuration examples": "Ships in bottles (examples)",
  "JavaScript and TypeScript": "A JavaScript and TypeScript ship",
  Python: "A Python ship",
  Monorepo: "Monorepo: one ship, many crews",
  "Custom steps": "Yer own hands (custom steps)",
  Reference: "Charts and tables",
  Configuration: "Configuration, the ship's charts",
  "Built-in linters": "The standing crew (builtins)",
  "Environment variables": "Standing orders (environment variables)",
  Glossary: "The crew's glossary",
  "CLI commands": "The bosun's calls (CLI)",
  Project: "The ship's company",
  Benchmarks: "Speed trials (benchmarks)",
  "About hk": "About hk: the bosun's tale",
  "Security model": "The ship's defences (security model)",
  Contributing: "Signing aboard to contribute",
  "Sea shanty": "The shanty",
};

/**
 * Marks up an entry's trailing English gloss, "Foul weather
 * (troubleshooting)", as `Foul weather <span class="hk-gloss">troubleshooting</span>`,
 * so that sea shanty mode sets it as a line of its own under the crew's
 * words (shanty-mode.css). VitePress renders sidebar text as HTML, here and
 * in the "Astern" and "Ahead" signposts at a page's foot.
 */
export function markGloss(text: string): string {
  const gloss = /^(.+?) \(([^()]+)\)$/.exec(text);
  return gloss ? `${gloss[1]} <span class="hk-gloss">${gloss[2]}</span>` : text;
}

/**
 * A sidebar with its words from `text` and its links moved under `prefix`,
 * except the links `moves` turns down, which keep their English page. A
 * translation's trailing gloss is marked up by `markGloss`.
 */
export function translateSidebar(
  items: SidebarItem[],
  prefix: string,
  text: Record<string, string>,
  moves: (link: string) => boolean = () => true,
): SidebarItem[] {
  return items.map((item) => ({
    ...item,
    text: item.text && (Object.hasOwn(text, item.text) ? markGloss(text[item.text]) : item.text),
    ...(item.link && moves(item.link) ? { link: `${prefix}${item.link}` } : {}),
    ...(item.items ? { items: translateSidebar(item.items, prefix, text, moves) } : {}),
  }));
}
