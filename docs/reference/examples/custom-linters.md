---
description: Define your own check and fix commands, test them, and add conditions or platform-specific scripts.
---

# Custom steps

A step can invoke any shell command. Define which files it uses, how to check them without writes, and how to apply fixes.

This example manually defines a whitespace step using an hk utility. It needs only hk, and includes tests you can run before adding the step to your workflow.

<a href="/custom-linters.pkl" download>Download custom-linters.pkl</a> and save it as `hk.pkl`.

## Configuration

<<< @/public/custom-linters.pkl

## Test the step

```sh
hk validate
hk test --step whitespace
hk check --all --plan
```

Each test writes a file in a temporary sandbox. One expects a clean check to succeed; the other checks the exact content after fixing. The `files` list explicitly selects the sandbox paths passed to each command.

Use this pattern when adding a custom linter or contributing a builtin.

## Add a condition

Conditions use expression syntax. To invoke a shell test, wrap it in `exec`:

```pkl
condition = "exec('test -f .lint-enabled')"
```

This fragment assumes a POSIX shell. `condition` is evaluated for each job; use `step_condition` to evaluate once for the step.

## Use platform-specific commands

For a project that provides both shell and PowerShell check scripts, define a `Script`:

```pkl
check = new Script {
  linux = "sh scripts/check.sh"
  macos = "sh scripts/check.sh"
  windows = "pwsh -NoProfile -File scripts/check.ps1"
}
```

These scripts are project-owned placeholders: create them before using the fragment. They must leave files unchanged when used as a check.

## Add optimizations when supported

- `check_list_files` reports only the files that need fixing.
- `check_diff` emits a unified diff that hk can apply.
- `batch = true` lets hk divide files among jobs when the tool supports independent subsets.
- `workspace_indicator` runs commands for matching projects.

Keep the step’s selected files consistent with everything the command can modify. For broader effects, use dependencies or `exclusive = true`. See the [configuration reference](/configuration).
