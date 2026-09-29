---
title: "hk util check-merge-conflict"
description: "Scan the cargo for merge markers, even when Git isn't in the middle of a merge."
sourceHash: ef42464cc17a
---

<!-- Pirate variant of docs/cli/util/check-merge-conflict.md; see docs/pirate/STYLE.md. -->

# `hk util check-merge-conflict`

- **How to hail it:** `hk util check-merge-conflict [--assume-in-merge] <FILES>…`
- **Effect:** read-only (looks, never touches)

Checks for merge conflict markers, the tangled rigging a conflicted merge leaves behind.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to check

## Flags to fly {#flags}

- **`--assume-in-merge`** — Run the check even when no merge is under way
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Scan for merge markers even when Git isn't merging right now.

```sh
hk util check-merge-conflict --assume-in-merge src/main.rs
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
