---
title: "hk util no-commit-to-branch"
description: "Guard a branch ye name, instead of the default main/master list."
sourceHash: e7d2e0301880
---

<!-- Pirate variant of docs/cli/util/no-commit-to-branch.md; see docs/pirate/STYLE.md. -->

# `hk util no-commit-to-branch`

- **How to hail it:** `hk util no-commit-to-branch [--branch <BRANCH>]`
- **Effect:** read-only (looks, never touches)

Prevents commits to specific branches: no voyage may set sail on a guarded branch.

## Flags to fly {#flags}

- **`--branch <BRANCH>`** — Branch names to guard (default: main, master)
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Guard a branch ye name instead of the default main/master list.

```sh
hk util no-commit-to-branch --branch production
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
