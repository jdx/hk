---
name: hk-configure
description: Configure hk git hooks and project checks in hk.pkl. Use when adding or changing linters, formatters, or custom steps in a project that uses hk, or when setting up hk at the user's request.
---

# Configure hk

Start with `hk --version`, the project's tool declarations, and the configuration hk actually selects. Check `HK_FILE` first: when set, it replaces the normal candidate list (a relative path is searched from the working directory upward). Otherwise, walk upward from the working directory, checking `hk.local.pkl`, `.config/hk.local.pkl`, `hk.pkl`, then `.config/hk.pkl` at each level. Legacy `hk.toml`, `hk.yaml`, `hk.yml`, and `hk.json` are checked after those Pkl paths at each level. The first existing file wins. Inspect a local override's `amends` chain before editing the shared project file.

Preserve the existing Pkl package version and hook structure unless the task includes upgrading them. `hk config sources` prints the precedence of configuration layers; it does not identify the selected project file. `hk config dump` shows merged settings.

For a new setup, `hk init` generates a configuration with package imports matching the installed hk. Do not use `--force` to replace an existing configuration. Installing Git hooks is separate from defining checks; use `hk install` when hook installation is part of the task.

## Add a check

Prefer the built-in definition when it fits: `hk builtins` lists available names. Builtins configure commands and file selection; the linter executable still needs to be available through the project's existing tool setup.

Extend the project's existing mapping rather than replacing its hooks. For example, with its existing `Builtins.pkl` import, add this entry to the linter mapping used by the relevant hooks:

```pkl
["prettier"] = (Builtins.prettier) {
  glob = List("**/*.js", "**/*.ts", "**/*.json")
}
```

Keep the `Config.pkl` and `Builtins.pkl` imports on the same package version. Do not copy a newer builtin's fields into a project pinned to an older schema without checking compatibility.

For a custom step, set `glob` to the intended files, `check` to the reporting command, and `fix` only when the tool supports correction. `{{files}}` passes the selected paths. In monorepos, inspect existing `dir` and `workspace_indicator` choices before adding a repository-wide command. Use `depends` for real ordering dependencies; independent steps can run concurrently.

## Verify the configuration and file selection

Run `hk validate`, then preview the relevant hook without executing its commands:

```sh
hk check --all --why prettier
```

Replace `prettier` with the added step and choose explicit files instead of `--all` when the task is narrower. If the step only exists in `pre-commit`, inspect it with `hk run pre-commit --plan --why prettier` instead; do not assume the `check` hook has identical steps.

After reviewing the selected commands and confirming their effects fit the task, run the targeted check:

```sh
hk check --check --step prettier --all
```

`--check` selects check commands even if the hook sets `fix = true`; it does not make arbitrary commands read-only. Custom check commands can write files. Use `hk fix` when corrections are intended and inspect the resulting diff; pre-commit fixers may also stage changes.

For less common fields, consult the [configuration reference](https://hk.jdx.dev/configuration), using the project's pinned schema and installed CLI help to resolve version differences.
