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
