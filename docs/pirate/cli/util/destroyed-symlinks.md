---
title: "hk util destroyed-symlinks"
description: "Search the whole ship, or narrow the check to the paths ye name."
sourceHash: 86b3d250c6f1
---

<!-- Pirate variant of docs/cli/util/destroyed-symlinks.md; see docs/pirate/STYLE.md. -->

# `hk util destroyed-symlinks`

- **How to hail it:** `hk util destroyed-symlinks [FILES]…`
- **Effect:** read-only (looks, never touches)

Checks for symlinks replaced by regular files containing their target path: the mooring line's been cut, and only its label is left.

## Cargo it takes {#arguments}

- **`[FILES]…`** — The files to check. Name none, and it checks the whole repository.

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Check the whole ship, or narrow the check to specific paths.

```sh
hk util destroyed-symlinks
hk util destroyed-symlinks docs/link.md
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
