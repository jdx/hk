---
title: "hk util"
description: "The ship's tool chest: utilities that work straight on the arguments ye hand them. Several also sail ready-rigged as builtins, the standing crew."
sourceHash: 770e0a8726ce
---

<!-- Pirate variant of docs/cli/util.md; see docs/pirate/STYLE.md. -->

# `hk util`

- **How to hail it:** `hk util <SUBCOMMAND>`
- **Effect:** read-only for this call itself, which only lists the lesser calls. Some of them mend the cargo in place. `format-diff` and `sarif-diff` run whatever command ye hand them, so they do whatever that command does; the others' pages each give their own effect.

Utility commands for file operations: the ship's tool chest for working the cargo.

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

## Lesser calls {#subcommands}

- [`hk util check-added-large-files [--maxkb <MAXKB>] <FILES>…`](/cli/util/check-added-large-files.md)
- [`hk util check-byte-order-marker [-d --diff] <FILES>…`](/cli/util/check-byte-order-marker.md)
- [`hk util check-case-conflict <FILES>…`](/cli/util/check-case-conflict.md)
- [`hk util check-conventional-commit [--allowed-types <ALLOWED_TYPES>] <COMMIT_MSG_FILE>`](/cli/util/check-conventional-commit.md)
- [`hk util check-executables-have-shebangs <FILES>…`](/cli/util/check-executables-have-shebangs.md)
- [`hk util check-merge-conflict [--assume-in-merge] <FILES>…`](/cli/util/check-merge-conflict.md)
- [`hk util check-shebang-scripts-are-executable <FILES>…`](/cli/util/check-shebang-scripts-are-executable.md)
- [`hk util check-symlinks <FILES>…`](/cli/util/check-symlinks.md)
- [`hk util destroyed-symlinks [FILES]…`](/cli/util/destroyed-symlinks.md)
- [`hk util detect-private-key <FILES>…`](/cli/util/detect-private-key.md)
- [`hk util end-of-file-fixer [-d --diff] [-f --fix] <FILES>…`](/cli/util/end-of-file-fixer.md)
- [`hk util fix-byte-order-marker <FILES>…`](/cli/util/fix-byte-order-marker.md)
- [`hk util fix-smart-quotes [--check] [-d --diff] <FILES>…`](/cli/util/fix-smart-quotes.md)
- [`hk util forbid-submodules [PATHS]…`](/cli/util/forbid-submodules.md)
- [`hk util format-diff [--no-stdin] <FILES>… <-- COMMAND>…`](/cli/util/format-diff.md)
- [`hk util mixed-line-ending [-d --diff] [-f --fix] <FILES>…`](/cli/util/mixed-line-ending.md)
- [`hk util no-commit-to-branch [--branch <BRANCH>]`](/cli/util/no-commit-to-branch.md)
- [`hk util python-check-ast <FILES>…`](/cli/util/python-check-ast.md)
- [`hk util python-debug-statements <FILES>…`](/cli/util/python-debug-statements.md)
- [`hk util sarif-diff [--findings-exit-code <CODE>] <-- COMMAND>…`](/cli/util/sarif-diff.md)
- [`hk util trailing-whitespace [-d --diff] [-f --fix] <FILES>…`](/cli/util/trailing-whitespace.md)

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Each utility works straight on the arguments ye hand it. Several also sail ready-rigged as builtins, part of the standing crew that comes with the ship.

```sh
hk util trailing-whitespace README.md
hk util end-of-file-fixer README.md
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
