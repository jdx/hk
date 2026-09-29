---
title: "hk run post-merge"
description: "The cargo it takes and the flags ye can fly with hk run post-merge."
sourceHash: 8dd1d26b02d8
---

<!-- Pirate variant of docs/cli/run/post-merge.md; see docs/pirate/STYLE.md. -->

# `hk run post-merge`

- **How to hail it:** `hk run post-merge [FLAGS] <IS_SQUASH> [FILES]…`

Run the post-merge hook, the bosun's pipe that calls all hands after a merge.

## Cargo it takes {#arguments}

- **`<IS_SQUASH>`** — A flag telling whether the merge was a squash merge (1) or not (0)
- **`[FILES]…`** — Run on specific files, just the cargo ye name

## Flags to fly {#flags}

- **`-a --all`** — Muster all tracked files and the eligible untracked ones, then apply the step filters to that cargo
- **`-c --check`** — Inspect the cargo: run the check command instead of the fix command
- **`-e --exclude <EXCLUDE>`** — Turn away (exclude) files that would otherwise have been selected
- **`-f --fix`** — Mend the canvas: run fix commands instead of check commands, for this call only
- **`-g --glob <GLOB>`** — Run on the cargo that matches these glob patterns
- **`-J --json`** — Write the passage plan out as JSON, when flown with --plan or --why
- **`-P --plan`** — Print the passage plan instead of running the hook
- **`-S --step <STEP>`** — Run only the step(s) ye name, and no other hands
- **`-W --why [STEP]`** — Show in detail why things were included or excluded. Name a step to focus on that one hand, or leave the value off to show the reasons for every step. Implies --plan.
- **`--fail-fast`** — Abort at the first failure: the first squall ends the run
- **`--files0-from <PATH>`** — Read the exact file list, the cargo manifest, from a NUL-delimited file, or from stdin with `-` (except for hooks that keep stdin for themselves)
- **`--format <FORMAT>`** — Choose the run's output: for human eyes, or machine-readable

  **Choose from:** `human`, `json`, `jsonl`

- **`--from-ref <FROM_REF>`** — Select the cargo changed since this reference; pair it with --to-ref if ye like
- **`--junit-xml <PATH>`** — Write each step's results into a JUnit XML report
- **`--no-fail-fast`** — Keep sailing through failures (the opposite of --fail-fast)
- **`--no-stage`** — Turn off auto-staging of fixed files: mended cargo is not loaded aboard for ye
- **`--pr`** — Inspect only the cargo changed in the current PR/branch (a shortcut for --from-ref DEFAULT_BRANCH --to-ref HEAD)
- **`--safe`** — Reject commands with unknown or destructive effects before they execute
- **`--sarif <PATH>`** — Write the normalized diagnostics (what the lookouts sang out) as SARIF
- **`--skip-step <STEP>`** — Skip the step(s) ye name; those hands sit this one out
- **`--stage`** — Turn on auto-staging of fixed files: mended cargo is loaded aboard for ye
- **`--staged`** — Run on staged files only, the cargo already loaded aboard, without stowing unstaged changes in the hold
- **`--stash <STASH>`** — How to stow the hold: the stash method to use for git hooks

  **Choose from:** `git`, `patch-file`, `none`

- **`--stats`** — Show statistics about the cargo matching each step
- **`--to-ref <TO_REF>`** — The end reference for the comparison with --from-ref
- **`--unstaged`** — Run on unstaged and untracked files only (staged files are left out), without stashing. Handy for linting files a clockwork hand (a coding agent) just changed.
- **`-h --help`** — Print help, matey

<!-- hk documentation examples -->

## Further charts {#learn-more}

[Git hooks and stowing the hold](/hooks) · [Troubleshooting and the ship's log](/logging) · [All the bosun's calls](/cli/)
