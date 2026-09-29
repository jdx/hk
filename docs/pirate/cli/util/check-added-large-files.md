---
title: "hk util check-added-large-files"
description: "Weigh the cargo against a size limit in kilobytes before it comes aboard."
sourceHash: f17581d463c7
---

<!-- Pirate variant of docs/cli/util/check-added-large-files.md; see docs/pirate/STYLE.md. -->

# `hk util check-added-large-files`

- **How to hail it:** `hk util check-added-large-files [--maxkb <MAXKB>] <FILES>…`
- **Effect:** read-only (looks, never touches)

Checks for large files being added to the repository: no hold-busting crates loaded aboard the ship.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to check

## Flags to fly {#flags}

- **`--maxkb <MAXKB>`** — The most a file may weigh, in kilobytes

  **Unless ye say otherwise:** `500`

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Weigh each file against a size limit in kilobytes, matey.

```sh
hk util check-added-large-files --maxkb 1024 assets/logo.png
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
