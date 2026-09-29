---
title: "hk util check-case-conflict"
description: "Compare the paths ye hand over for naming collisions once case is ignored, and sing out at any clash."
sourceHash: 4e96c6d51944
---

<!-- Pirate variant of docs/cli/util/check-case-conflict.md; see docs/pirate/STYLE.md. -->

# `hk util check-case-conflict`

- **How to hail it:** `hk util check-case-conflict <FILES>…`
- **Effect:** read-only (looks, never touches)

Checks the cargo for case-insensitive filename conflicts: two names that differ only in their capitals.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to check for case conflicts

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Compare the paths ye hand over for naming collisions that ignore case.

```sh
hk util check-case-conflict src/App.ts src/app.ts
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
