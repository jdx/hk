---
description: Why sail with hk? Its lashings (file locks), how it works each lookout's strengths, the charts it steers by, and the tradeoffs ye take aboard.
sourceHash: 9f8ec8b3829f
---

# Why sail with hk?

hk be built for ships that carry several linters and formatters over the same cargo. It sends the crew aloft all at once and lashes each file as they work, so the hands haul together without two of them writing the same file at the same moment.

## All hands at once need lashings {#parallelism-needs-coordination}

A read-only check can run alongside any other check: lookouts only look. A formatter needs a file to itself while it mends it. Without lashings, two formatters can read the same original canvas and each stitch over the other's fixes.

For each run of a hook, hk keeps read/write locks on the files each step has been handed:

| The work                             | The lashing       | Who can haul alongside?                   |
| ------------------------------------ | ----------------- | ----------------------------------------- |
| Inspect a file (check)               | Read              | Any other hand reading that file          |
| Mend a file (fix)                    | Write             | Hands working on different files          |
| Check or fix cargo that isn't shared | Independent locks | The rest of the crew, up to the job limit |

This holds only if the steps are charted true. A check must be read-only, and a step must declare every file its commands may touch. A command that changes files it never declared slips its lashings, and hk can't protect ye. Use `depends` or `exclusive` when a tool's reach goes beyond the files it was handed.

## Work each lookout to its strengths {#use-each-linter-s-capabilities}

Sailmakers on different files mend at once. A fix holds write locks on its step's files, so only two who share a sail take it in turns. A step that only has a `check` command declaring `effect = "read"` changes no files, so even under `hk fix` it holds read locks and looks over the same sail alongside other hands that only read it; it still waits for a fixer that mends the sail. A check that does not declare a read effect, such as the builtin type checkers, whose caches may be written, holds write locks. hk's [builtins](/builtins) know quicker ways to work the tools that allow it.

### Diff output {#diff-output}

A `check_diff` command writes out a patch and leaves the files be. When fixing, hk runs it and applies the patch itself instead of running `fix`. If the patch won't take, hk runs `fix` instead, or, for a step with no `fix`, the command it runs when checking. When the command declares `effect = "read"`, as most builtins do, hk works out the patch under read locks, so the formatter hauls alongside other steps reading the same files. It takes write locks only on the files the patch changes, and only long enough to apply it. If another hand changed one of those files in the meantime, hk works out the patch again under write locks. A `check_diff` that declares no read effect holds its write locks the whole way, as any fix does. Builtins such as Ruff's formatter sail this way.

### Lists of files needing fixes {#lists-of-files-needing-fixes}

A `check_list_files` command calls out which files need mending. When the step checks first, as below, Prettier's `--list-different` lets hk hand only those files to `--write`.

### Check before fixing {#check-before-fixing}

For other tools, a step can set `check_first = true` to inspect before it mends, and skip the mending when the inspection passes. When files do need mending, the tool runs twice, and in hk's benchmark that cost more than it saved, so it's off unless ye ask for it.

These tactics change how much work the orchestration costs. How fast the crew actually hauls depends on yer linters, how much cargo they share, how many files changed, and how many CPU cores ye have. See the [benchmarks](/benchmarks) for a workload ye can reproduce, and its limitations.

## Set sail with part of the cargo {#work-with-partial-commits}

When a pre-commit hook uses `stash = "git"`, hk stows yer unstaged changes in the hold for a spell, runs the steps against the staged versions, and brings the stowed work back up afterward.

The unit a linter works on is a **file**, not a staged hunk. Stage one function, and a formatter can still reformat the whole staged version of that file. Stashing keeps unrelated work out of that version; it does not make the formatter work hunk by hunk.

Read [hooks and stowing the hold](/hooks#stashing-and-partial-commits) for automatic staging, reviewing fixes before ye commit, and what to do if the stowed work won't come back.

## Share the charts, keep command of yer tools {#reuse-configuration-keep-control-of-tools}

Builtins are Pkl step definitions: file patterns and commands ye can inspect and amend. They call on tools already aboard yer environment. hk also carries [native utilities](/cli/util) for jobs such as trailing whitespace and merge conflict checks.

Pkl imports can point at local files or remote packages. Pin package versions, read over any configuration ye import, and manage linter versions as ye would any other project dependency. A configuration that defines a shell command decides what hk will execute, so know what ye're signing.

## Is hk the ship for ye? {#is-hk-a-fit}

hk serves ye well when ye want:

- The same steps in Git hooks, local checks, and CI.
- Checks hauling at once, and fixes coordinated over shared files.
- Reusable configuration with types, imports, and local overrides.
- Command over how tools are brought aboard, through mise or the package manager ye already use.

The price o' passage: a configuration language to learn, and the job of providing yer own tools. File locks keep the hands from colliding; they cannot make peace between formatters with clashing style rules. Choose rules that agree, or use `depends` to set the order yer project needs.

## Changing ships from another hook manager {#moving-from-another-hook-manager}

Ye can try hk on a branch before changing how yer shipmates work. Write a configuration, run `hk check --all --plan`, then compare its checks and fixes with the workflow ye have now.

For a pre-commit or prek configuration, start with [`hk migrate pre-commit`](/cli/migrate/pre-commit). Hooks it knows become hk builtins, and local shell hooks become hk steps. Everything else keeps running through prek or pre-commit, so ye can come aboard now and convert the rest later.

[Get under way with getting started](/getting_started) or browse the [ships in bottles, the configuration examples](/reference/examples/).
