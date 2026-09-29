---
title: "hk migrate"
description: "Change ships: convert a pre-commit configuration, then look over the generated file and the tools it needs aboard."
sourceHash: d84290e6d18f
---

<!-- Pirate variant of docs/cli/migrate.md; see docs/pirate/STYLE.md. -->

# `hk migrate`

- **How to hail it:** `hk migrate <SUBCOMMAND>`
- **Effect:** read-only (looks, never touches)

Change ships: migrate from other hook managers to hk.

## Flags to fly {#flags}

- **`-h --help`** — Print help, matey

## Lesser calls {#subcommands}

- [`hk migrate pre-commit [FLAGS]`](/cli/migrate/pre-commit.md)

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Convert a pre-commit configuration, then look over the generated file and the tools it needs aboard.

```sh
hk migrate pre-commit --output hk.migrated.pkl
HK_FILE=./hk.migrated.pkl hk validate
```

## Further charts {#learn-more}

[Set sail with getting started](/getting_started) · [Troubleshooting and the ship's log](/logging) · [All the bosun's calls](/cli/)
