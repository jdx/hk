---
title: "hk fix"
description: "Fix commands may also stage their changes, loading the mended cargo aboard. Use --no-stage to leave fixes unstaged, then review both the working-tree and staged diffs."
sourceHash: 9f18b9e71d17
---

<!-- Pirate variant of docs/cli/fix.md; see docs/pirate/STYLE.md. -->

# `hk fix`

- **How to hail it:** `hk fix [FLAGS] [FILES]…`
- **Also answers to:** `f`

Run the fix hook: the sailmakers mend the canvas.

Each step runs its fix command, which modifies files in place. Pass `--check`, or set the standing order `HK_CHECK=1` or `HK_FIX=0`, and each step runs its check command instead: inspection only, no mending.

## Cargo it takes {#arguments}

- **`[FILES]…`** — Run on specific files: just the cargo ye name

## Flags to fly {#flags}

- **`-a --all`** — Select all tracked and eligible untracked files, the whole of the cargo, then apply step filters
- **`-c --check`** — Run the check command instead of the fix command: inspect the cargo, don't mend it
- **`-e --exclude <EXCLUDE>`** — Exclude files that otherwise would have been selected; that cargo stays out of the run
- **`-f --fix`** — Run fix commands instead of check commands for this call only: mend the canvas
- **`-g --glob <GLOB>`** — Run on files that match these glob patterns
- **`-J --json`** — Output the passage plan as JSON when combined with --plan or --why
- **`-P --plan`** — Print the passage plan instead of running the hook
- **`-S --step <STEP>`** — Run only specific step(s): call just the hands ye name
- **`-W --why [STEP]`** — Show detailed reasons for inclusion/exclusion: why each hand was mustered or left ashore. Pass a step name to focus on one step, or leave the value off to show reasons for all steps. Implies --plan.
- **`--fail-fast`** — Abort on the first failure: the first squall ends the run
- **`--files0-from <PATH>`** — Read the exact file list, the cargo manifest, from a NUL-delimited file, or from stdin with `-` (except hooks that reserve stdin)
- **`--format <FORMAT>`** — Select human or machine-readable execution output: a log for a sailor's eyes, or one for machines

  **Choose from:** `human`, `json`, `jsonl`

- **`--from-ref <FROM_REF>`** — Select files changed since this reference; optionally pair it with --to-ref
- **`--junit-xml <PATH>`** — Write each step's results as a JUnit XML report
- **`--no-fail-fast`** — Continue on failures, sailing on through every squall (opposite of --fail-fast)
- **`--no-stage`** — Disable auto-staging of fixed files: mended cargo stays on the dock, unstaged
- **`--pr`** — Check only files changed in the current PR/branch (shortcut for --from-ref DEFAULT_BRANCH --to-ref HEAD)
- **`--safe`** — Reject commands with unknown or destructive effects before anything runs
- **`--sarif <PATH>`** — Write normalized diagnostics as SARIF
- **`--skip-step <STEP>`** — Skip specific step(s): those hands sit this one out
- **`--stage`** — Enable auto-staging of fixed files: mended cargo is loaded aboard
- **`--staged`** — Run on staged files only, without stashing unstaged changes: just the cargo loaded aboard, with nothing stowed in the hold
- **`--stash <STASH>`** — Stash method to use for git hooks: how the unstaged cargo is stowed in the hold

  **Choose from:** `git`, `patch-file`, `none`

- **`--stats`** — Display statistics about the files matching each step
- **`--to-ref <TO_REF>`** — End reference for comparison with --from-ref
- **`--unstaged`** — Run on unstaged and untracked files only (excludes staged files), without stashing. Useful for linting files a clockwork hand (a coding agent) just changed.
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Fix commands may also stage their changes, loading the mended cargo aboard. Use --no-stage to leave the fixes unstaged, on the dock, then review both the working-tree and the staged diffs.

```sh
hk fix --no-stage
hk fix --all --no-stage
hk fix --step prettier
git diff
git diff --cached
```

## Further charts {#learn-more}

[Getting started: set sail](/getting_started) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
