---
name: hk-debug
description: Diagnose failing or unexpectedly skipped hk checks and git hooks, including file selection, configuration overrides, and partially staged files. Use when a project using hk has a hook failure or a step runs on the wrong files.
---

# Diagnose hk checks and hooks

Capture the failing command, `hk --version`, and `git status --short`. Distinguish a configuration error, an excluded step, a missing executable, and an error from the linter itself. Read the first substantive error from the failing step, not just hk's final exit status.

## Inspect the effective configuration

Check `HK_FILE` first: when set, it replaces the normal candidate list (a relative path is searched from the working directory upward). Otherwise, walk upward from the working directory, checking `hk.local.pkl`, `.config/hk.local.pkl`, `hk.pkl`, then `.config/hk.pkl` at each level. Legacy `hk.toml`, `hk.yaml`, `hk.yml`, and `hk.json` are checked after those Pkl paths at each level. The first existing file wins. Inspect a local override's `amends` chain before editing the shared project file.

`hk config sources` prints the static precedence of configuration layers; `hk config dump` shows merged settings. Neither identifies the selected project file. Inspect relevant `HK_*` overrides and use `hk config explain skip_steps` (or another relevant setting) when a setting differs from the project file.

Run `hk validate` for Pkl evaluation or schema errors. Compare the project's pinned package imports with its installed hk version before suggesting an upgrade.

## Explain skipped steps and files

Preview the same hook and file-selection flags as the failing invocation:

```sh
hk run pre-commit --plan --why prettier
hk check --all --why prettier
```

These are different scopes: the second command is useful as a comparison, not a reproduction of a staged-file failure. Replace `prettier` with the actual step. `--why` implies a plan and explains inclusion and exclusion; `--json` makes that plan machine-readable.

Check the selected paths against `glob`, `exclude`, `dir`, `workspace_indicator`, profiles, conditions, and skipped-step settings. `hk check` and `hk run pre-commit` may have different step mappings and fix/stash settings. A successful all-files check does not prove the staged snapshot will pass.

## Reproduce the failing step

Once the plan identifies the right files and command, narrow execution to that step. For example:

```sh
hk check --check --step prettier path/to/file.ts
```

`--check` chooses check commands, but a custom check can still write files. Inspect it before executing. For command-not-found errors, check the configured `prefix`, working directory, and project tool environment rather than installing a different global tool. For linter errors, fix the reported code or configuration and rerun the same scope.

For partially staged files, inspect both `git diff` and `git diff --cached`. hk may stash unstaged changes, run fixers, stage their output, then restore the unstaged work. If restoration fails, preserve hk's recovery message and the current index/worktree; do not drop stashes or reset files to make the error disappear. Resolve the specific conflict while preserving both sets of changes.

Do not disable the hook or add a skip setting as a substitute for fixing the cause unless the user asks for that behavior. Report the cause, the targeted change, and which original invocation now succeeds.

See the [hook lifecycle](https://hk.jdx.dev/hooks) and [CLI reference](https://hk.jdx.dev/cli/) for additional diagnostics; prefer the installed version's `--help` when flags differ.
