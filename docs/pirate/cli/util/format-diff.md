---
title: "hk util format-diff"
description: "The cargo and flags for hk util format-diff, which turns a sailmaker's work into a patch."
sourceHash: 20ddeda12bc2
---

<!-- Pirate variant of docs/cli/util/format-diff.md; see docs/pirate/STYLE.md. -->

# `hk util format-diff`

- **How to hail it:** `hk util format-diff [--no-stdin] <FILES>… <-- COMMAND>…`

Prints a patch of what a sailmaker (formatter) would change, for a `check_diff` command.

Runs COMMAND once for each file, with the file on stdin, and compares what it prints with the file. `{}` in COMMAND is replaced with the file's path, for formatters that take the path of their input as an option. With `--no-stdin`, COMMAND reads the file itself, from the path `{}` gives it. Files are formatted in parallel: all hands haul at once.

Prints a unified diff for every file that would change and exits 1, or exits 0 when none would. If COMMAND fails for any file, or prints nothing for a file that isn't empty, no patch is printed at all: its error output is shown, and hk runs the step's fixer instead to mend the canvas.

For example: `hk util format-diff a.lua b.lua -- stylua --stdin-filepath {} -`

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to format
- **`<-- COMMAND>…`** — The sailmaker (formatter), reading stdin and writing the formatted file to stdout

## Flags to fly {#flags}

- **`--no-stdin`** — Give COMMAND no stdin, for a formatter that reads the file at `{}` itself
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
