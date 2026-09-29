---
title: "hk config dump"
description: "Have the runtime settings in force written out as JSON or TOML, whichever ye choose."
sourceHash: 8b8f475fe321
---

<!-- Pirate variant of docs/cli/config/dump.md; see docs/pirate/STYLE.md. -->

# `hk config dump`

- **How to hail it:** `hk config dump [--format <FORMAT>]`
- **Effect:** read-only (looks, never touches)

Print the effective runtime settings, the ones the ship is sailing by right now.

Lays out the configuration once every source that came aboard has been merged, including CLI flags, environment variables, git config, project config, and user config.

## Flags to fly {#flags}

- **`--format <FORMAT>`** — The format to write it out in

  **Choose from:** `json`, `toml`

  **Unless ye say otherwise:** `json`

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Pick JSON or TOML for the runtime settings in force, matey.

```sh
hk config dump
hk config dump --format toml
```

## Further charts {#learn-more}

[Configuration guide: the ship's charts](/configuration) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
