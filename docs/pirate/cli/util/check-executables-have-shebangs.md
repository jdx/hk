---
title: "hk util check-executables-have-shebangs"
description: "Make sure an executable script flies its shebang."
sourceHash: bbb4ac4b63d7
---

<!-- Pirate variant of docs/cli/util/check-executables-have-shebangs.md; see docs/pirate/STYLE.md. -->

# `hk util check-executables-have-shebangs`

- **How to hail it:** `hk util check-executables-have-shebangs <FILES>…`
- **Effect:** read-only (looks, never touches)

Checks that executable files have shebangs: any file marked to run must fly its colours.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to check

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Check that an executable script flies its shebang.

```sh
hk util check-executables-have-shebangs scripts/check.sh
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
