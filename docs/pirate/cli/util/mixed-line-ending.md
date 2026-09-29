---
title: "hk util mixed-line-ending"
description: "Look for mixed line endings, or bring them all into line as LF."
sourceHash: 91569b5dea8d
---

<!-- Pirate variant of docs/cli/util/mixed-line-ending.md; see docs/pirate/STYLE.md. -->

# `hk util mixed-line-ending`

- **How to hail it:** `hk util mixed-line-ending [-d --diff] [-f --fix] <FILES>…`
- **Effect:** modifies state (it touches, as well as looks)

Spots mixed line endings in the cargo, and mends them if ye ask.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to inspect or mend

## Flags to fly {#flags}

- **`-d --diff`** — Write out a diff of the change. Cannot be used with `fix`.
- **`-f --fix`** — Mend mixed line endings by normalizing them to LF
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Inspect the cargo for mixed line endings, or normalize them all to LF.

```sh
hk util mixed-line-ending README.md
hk util mixed-line-ending --fix README.md
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
