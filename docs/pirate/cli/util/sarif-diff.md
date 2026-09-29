---
title: "hk util sarif-diff"
description: "The cargo and flags for hk util sarif-diff, which turns the fixes in a lookout's SARIF report into a patch."
sourceHash: 51a2700d80b8
---

<!-- Pirate variant of docs/cli/util/sarif-diff.md; see docs/pirate/STYLE.md. -->

# `hk util sarif-diff`

- **How to hail it:** `hk util sarif-diff [--findings-exit-code <CODE>] <-- COMMAND>…`

Prints a patch from the fixes in a tool's SARIF report, for a `check_diff` command.

Runs COMMAND, reads the SARIF log it prints on stdout (the lookout's report), and applies the fixes its results carry to the files they name, printing a unified diff of the changes. Exits 0 when there are no results, and 1 after printing the patch when every result has a fix.

If any result has no fix, a fix can't be applied, or COMMAND fails, no patch is printed: the results or errors are shown and hk runs the step's fixer instead to mend the canvas. That way an unfixable finding is never hidden behind a patch that fixed the rest. COMMAND fails when it exits with anything but 0 or a findings exit code, which is 1 unless `--findings-exit-code` says otherwise.

For example: `hk util sarif-diff -- pinact run --check --format sarif a.yml`

## Cargo it takes {#arguments}

- **`<-- COMMAND>…`** — The tool (the lookout), printing a SARIF log on stdout

## Flags to fly {#flags}

- **`--findings-exit-code <CODE>`** — An exit code COMMAND uses for "found problems" (a lookout singing out) rather than for failing (repeatable; default 1)
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
