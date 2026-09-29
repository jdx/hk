---
title: "hk util python-check-ast"
description: "Check Python syntax with python3 (or python). If neither interpreter is available aboard, the utility skips the files. Python may create `__pycache__` directories."
sourceHash: 41d35aca3f95
---

<!-- Pirate variant of docs/cli/util/python-check-ast.md; see docs/pirate/STYLE.md. -->

# `hk util python-check-ast`

- **How to hail it:** `hk util python-check-ast <FILES>…`
- **Effect:** read-only (looks, never touches)

Checks Python files for valid syntax: every plank sound before she sails.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to check

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Checks Python syntax using python3 (or python). If neither interpreter is available aboard, the utility skips the files. Python may create `__pycache__` directories.

```sh
hk util python-check-ast app.py
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
