---
title: "hk util check-symlinks"
description: "Check where the symbolic links ye hand over are pointing."
sourceHash: 967677662487
---

<!-- Pirate variant of docs/cli/util/check-symlinks.md; see docs/pirate/STYLE.md. -->

# `hk util check-symlinks`

- **How to hail it:** `hk util check-symlinks <FILES>…`
- **Effect:** read-only (looks, never touches)

Checks for broken symlinks: lines run out to a mooring that isn't there.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to check

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Check the targets of the symbolic links ye hand over.

```sh
hk util check-symlinks bin/tool
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
