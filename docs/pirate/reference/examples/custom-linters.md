---
description: Sign on hands of yer own. Define yer own check and fix commands, put them through their paces, and add conditions or platform-specific scripts.
sourceHash: 24c70cd106e1
---

# Custom steps: signing on yer own hands

A step, one hand of the crew, can run any shell command ye like. Ye tell it which files it works on, how to inspect them without writing a thing, and how to mend them.

This example defines a whitespace step by hand, using one of hk's own utilities. It needs nothing aboard but hk, and it carries tests ye can run before ye add the step to yer workflow.

<a href="/custom-linters.pkl" download>Download custom-linters.pkl</a> and save it as `hk.pkl`, the charts yer ship sails by.

## The charts {#configuration}

<<< @/public/custom-linters.pkl

## Put the new hand through its paces {#test-the-step}

```sh
hk validate
hk test --step whitespace
hk check --all --plan
```

Each test writes a file in a temporary sandbox, a little ship all its own. One expects a clean check to succeed; the other checks the exact content of the file after the fix. The `files` list spells out exactly which sandbox paths each command is handed.

Follow this pattern when ye post a custom linter, a lookout of yer own, or contribute a builtin to the standing crew.

## Only when the wind's right: add a condition {#add-a-condition}

Conditions are written in expression syntax. To haul the step only when a shell test succeeds, wrap it in `exec_ok`:

```pkl
condition = "exec_ok('test -f .lint-enabled')"
```

This fragment assumes a POSIX shell aboard. `condition` is evaluated for each job; use `step_condition` to evaluate it just once for the step.

## Different waters, different commands: platform-specific scripts {#use-platform-specific-commands}

When yer project carries both shell and PowerShell check scripts, define a `Script`:

```pkl
check = new Script {
  linux = "sh scripts/check.sh"
  macos = "sh scripts/check.sh"
  windows = "pwsh -NoProfile -File scripts/check.ps1"
}
```

These scripts are placeholders that belong to yer project: create them before ye use the fragment. Used as a check, they must leave the files unchanged. A lookout only looks.

## Trim the sails: add optimizations when the tool allows {#add-optimizations-when-supported}

- `check_list_files` calls out only the files that need fixing.
- `check_diff` writes out a unified diff that hk can apply.
- `batch = true` lets hk divide the cargo among jobs that haul side by side, when the tool can work on independent subsets.
- `workspace_indicator` runs the commands for each matching project.

Keep the files a step is handed in line with everything its command can change: hk lashes only the files the step is handed. When a command reaches further than that, use dependencies or `exclusive = true`. See the [configuration reference](/configuration) for every setting the charts can hold.
