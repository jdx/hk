---
title: "hk agent instructions"
description: "Print the project's instructions for a clockwork hand without editing any agent configuration."
sourceHash: 2d724866c48e
---

<!-- Pirate variant of docs/cli/agent/instructions.md; see docs/pirate/STYLE.md. -->

# `hk agent instructions`

- **How to hail it:** `hk agent instructions <--target <TARGET>>`
- **Effect:** read-only (looks, never touches)

Print project instructions for a coding agent, the briefing for a clockwork hand coming aboard

## Flags to fly {#flags}

- **`--target <TARGET>`** — The agent to write the instructions for

  **Choose from:** `codex`, `claude-code`, `generic`

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Print project instructions without laying a finger on any agent configuration.

```sh
hk agent instructions --target codex
hk agent instructions --target generic
```

## Further charts {#learn-more}

[Coding agents: the clockwork hands](/agents) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
