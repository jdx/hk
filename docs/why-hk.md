---
description: Understand hk’s file locking, linter integration, configuration model, and tradeoffs.
---

# Why hk?

hk is designed for projects that run several linters and formatters over overlapping files. It combines concurrent execution with file-level coordination, so tools can work together without writing to the same file at the same time.

## Parallelism needs coordination

A read-only check can run alongside other checks. A formatter needs exclusive access to the files it changes. Without that coordination, two formatters can read the same original content and overwrite one another’s fixes.

Within a hook run, hk tracks read/write locks for the files selected by each step:

| Work                         | Lock              | What can run alongside it?       |
| ---------------------------- | ----------------- | -------------------------------- |
| Check a file                 | Read              | Other readers of that file       |
| Fix a file                   | Write             | Steps using different files      |
| Check or fix unrelated files | Independent locks | Other steps, up to the job limit |

This relies on accurate step definitions. A check must be read-only, and a step must declare the files its commands may touch. Commands that modify undeclared files bypass that protection. Use `depends` or `exclusive` when a tool’s effects extend beyond its selected files.

## Use each linter’s capabilities

Taking a write lock on every file can serialize otherwise independent work. hk’s [builtins](/builtins) describe more efficient ways to run tools when they support them.

### Diff output

A `check_diff` command emits a patch without editing files. hk can run it with read locks, then apply the patch under write locks. Builtins such as Ruff’s formatter use this approach.

### Lists of files needing fixes

A `check_list_files` command reports which files need changes. For example, Prettier’s `--list-different` lets hk narrow the files passed to `--write`.

### Check before fixing

For other tools, `check_first` can run a read-only check before acquiring write locks for a fix. When checks frequently pass, this avoids unnecessary exclusive access. When nearly every file needs fixing, the extra check may cost more than it saves.

These strategies affect orchestration overhead. Actual speed depends on your linters, file overlap, number of changed files, and available CPU cores. See the [benchmarks](/benchmarks) for a reproducible workload and its limitations.

## Work with partial commits

When a pre-commit hook uses `stash = "git"`, hk temporarily saves unstaged changes, runs the steps against the staged versions, and restores the saved work afterward.

The unit of linting is a **file**, not a staged hunk. If you stage one function, a formatter can still reformat the whole staged version of that file. Stashing keeps unrelated work out of that version; it does not make the formatter operate on individual hunks.

Read [hooks and stashing](/hooks#stashing-and-partial-commits) for automatic staging, review-before-commit settings, and recovery guidance.

## Reuse configuration, keep control of tools

Builtins are Pkl step definitions: file patterns and commands you can inspect and amend. They invoke tools available in your environment. hk also includes [native utilities](/cli/util) for tasks such as trailing whitespace and merge conflict checks.

Pkl imports can refer to local files or remote packages. Pin package versions, review imported configuration, and manage linter versions as you would other project dependencies. A configuration that defines a shell command determines what hk will execute.

## Is hk a fit?

hk is useful when you want:

- The same steps in Git hooks, local checks, and CI.
- Concurrent checks and coordinated fixes over shared files.
- Reusable configuration with types, imports, and local overrides.
- Control over tool installation through mise or an existing package manager.

The tradeoffs are a configuration language to learn and responsibility for providing your tools. File locks coordinate access; they cannot reconcile formatters with incompatible style rules. Choose compatible rules or use `depends` to establish the order your project needs.

## Moving from another hook manager

You can evaluate hk on a branch before changing your team’s setup. Create a configuration, run `hk check --all --plan`, then compare the checks and fixes with your existing workflow.

For a pre-commit configuration, start with [`hk migrate pre-commit`](/cli/migrate/pre-commit). Review the generated steps, tool versions, file filters, and any unsupported hooks before installing hk’s Git hooks.

[Get started](/getting_started) or browse the [configuration examples](/reference/examples/).
