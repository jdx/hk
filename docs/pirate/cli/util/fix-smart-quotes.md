---
title: "hk util fix-smart-quotes"
description: "This utility mends by default, matey; use --check to inspect without editing."
sourceHash: a804fb65cb75
---

<!-- Pirate variant of docs/cli/util/fix-smart-quotes.md; see docs/pirate/STYLE.md. -->

# `hk util fix-smart-quotes`

- **How to hail it:** `hk util fix-smart-quotes [--check] [-d --diff] <FILES>…`
- **Effect:** modifies state (lays hands on the cargo)

Replaces smart (curly) quotes with plain ASCII quotes, straight as a mast.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to inspect or mend (check/fix)

## Flags to fly {#flags}

- **`--check`** — Inspect for smart quotes without mending them
- **`-d --diff`** — Write out a diff of the change (implies `--check`)
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Mind ye, this utility mends by default; use --check to inspect without editing.

```sh
hk util fix-smart-quotes --check README.md
hk util fix-smart-quotes README.md
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
