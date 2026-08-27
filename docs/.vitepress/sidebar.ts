import spec from '../cli/commands.json' with { type: 'json' }

interface Command {
  subcommands: Record<string, Command & { hide?: boolean; full_cmd: string[] }>
}

export interface SidebarItem {
  text: string
  link?: string
  items?: SidebarItem[]
}

/** Return every visible command path in depth-first CLI order. */
function getCommands(cmd: Command): string[][] {
  const commands: string[][] = []
  for (const sub of Object.values(cmd.subcommands)) {
    if (sub.hide) continue
    commands.push(sub.full_cmd)
    commands.push(...getCommands(sub))
  }
  return commands
}

const commands = getCommands(spec.cmd)

// Shared by VitePress and the llms.txt generator so both expose the same pages.
export const sidebar: SidebarItem[] = [
  { text: 'Getting Started', link: '/getting_started' },
  { text: 'Configuration', link: '/configuration' },
  {
    text: 'Guides',
    items: [
      { text: 'Built-in Linters', link: '/builtins' },
      { text: 'Configuration Examples', link: '/reference/examples/' },
      { text: 'Git Hooks', link: '/hooks' },
      { text: 'mise Integration', link: '/mise_integration' },
      { text: 'Coding Agents', link: '/agents' },
    ],
  },
  {
    text: 'CLI Reference',
    link: '/cli/',
    items: commands.map((cmd) => ({
      text: cmd.join(' '),
      link: `/cli/${cmd.join('/')}`,
    })),
  },
  {
    text: 'Reference',
    items: [
      { text: 'Environment Variables', link: '/environment_variables' },
      { text: 'Pkl Introduction', link: '/pkl_introduction' },
      { text: 'Logging and Debugging', link: '/logging' },
      { text: 'Glossary', link: '/glossary' },
    ],
  },
  {
    text: 'Project',
    items: [
      { text: 'Why hk?', link: '/why-hk' },
      { text: 'Benchmarks', link: '/benchmarks' },
      { text: 'Contributing', link: '/contributing' },
      { text: 'About', link: '/about' },
      { text: 'Sea Shanty', link: '/shanty' },
    ],
  },
]
