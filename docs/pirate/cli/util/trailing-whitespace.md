---
title: "hk util trailing-whitespace"
description: "Inspect first, or cut away trailing whitespace in place."
sourceHash: d8ba2d120f22
---

<!-- Pirate variant of docs/cli/util/trailing-whitespace.md; see docs/pirate/STYLE.md. -->

# `hk util trailing-whitespace`

- **How to hail it:** `hk util trailing-whitespace [-d --diff] [-f --fix] <FILES>…`
- **Effect:** modifies state (it touches, as well as looks)

Checks for trailing whitespace, and mends it if ye ask: no loose threads left hanging off the end of a line.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to inspect or mend (check/fix)

## Flags to fly {#flags}

- **`-d --diff`** — Write out a diff of the change. Cannot be used with `fix`.
- **`-f --fix`** — Mend trailing whitespace by cutting it away
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Inspect first, or remove trailing whitespace in place.

```sh
hk util trailing-whitespace README.md
hk util trailing-whitespace --fix README.md
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
