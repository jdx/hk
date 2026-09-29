---
title: "hk util check-shebang-scripts-are-executable"
description: "Check that a script flying a shebang is marked executable, fit to run."
sourceHash: 84224633fd72
---

<!-- Pirate variant of docs/cli/util/check-shebang-scripts-are-executable.md; see docs/pirate/STYLE.md. -->

# `hk util check-shebang-scripts-are-executable`

- **How to hail it:** `hk util check-shebang-scripts-are-executable <FILES>…`
- **Effect:** read-only (looks, never touches)

Checks that files with shebangs are executable: a file that flies its colours had best be ready to run.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to check

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Check that a script flying a shebang is executable.

```sh
hk util check-shebang-scripts-are-executable scripts/check.sh
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
