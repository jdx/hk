---
title: "hk agent"
description: "Write out integration snippets for yer clockwork hands, to look over before ye add them to an agent host."
sourceHash: cc3be414f105
---

<!-- Pirate variant of docs/cli/agent.md; see docs/pirate/STYLE.md. -->

# `hk agent`

- **How to hail it:** `hk agent <SUBCOMMAND>`
- **Effect:** read-only (looks, never touches)

Generate integration snippets for coding agents, the clockwork hands who sail with ye

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

## Lesser calls {#subcommands}

- [`hk agent hooks <--target <TARGET>>`](/cli/agent/hooks.md)
- [`hk agent instructions <--target <TARGET>>`](/cli/agent/instructions.md)
- [`hk agent mcp <--target <TARGET>>`](/cli/agent/mcp.md)

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Generate integration snippets, and look them over before ye add them to an agent host.

```sh
hk agent instructions --target codex
hk agent hooks --target claude-code
hk agent mcp --target vscode
```

## Further charts {#learn-more}

[Coding agents: the clockwork hands](/agents) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
