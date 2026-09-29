---
description: Troubleshooting in foul weather. Diagnose hands that never turn out, charts that read wrong, missing tools, hook failures, and slow passages.
sourceHash: bff0d03d0bbb
---

# Troubleshooting: riding out foul weather

When the weather turns foul, start with the passage plan and verbose output. Between them they usually show whether the trouble lies in which cargo was picked (file selection), in the configuration, or in a linter command.

```sh
hk check --plan
hk check -v
```

## A hand never turns out {#a-step-does-not-run}

Ask hk to account for the step:

```sh
hk check --why eslint
hk check --all --why eslint
```

Swap `eslint` for the step's name as yer configuration writes it. Then look over its file patterns, exclusions, required environment variables, conditions, and profiles.

`hk check` normally picks the modified files. `--all` widens the selection. A step on the `slow` watch (profile) needs `--slow` or `--profile slow`; a step with several positive profiles requires every one of them.

To read the passage plan as JSON:

```sh
hk check --all --plan --json
```

A plan runs no linter commands. Evaluating the configuration and the conditions may still look about the environment, mind.

## The charts don't read as ye expect {#configuration-is-not-what-you-expect}

```sh
hk validate -v
hk config dump
hk config get skip_steps
hk config explain jobs
```

`hk validate` checks the Pkl configuration. `hk config dump` shows the settings in force at runtime, not the complete hook and step definitions. `hk config explain` helps ye find what overrode what.

Look for `hk.local.pkl`, a user config, Git settings, and `HK_*` environment variables (the standing orders), any of which can change the course. See [who outranks whom: configuration precedence](/configuration#configuration-precedence).

If the evaluator or the cache be giving ye trouble, bypass the resolved configuration cache and turn on debug logs:

```sh
HK_CACHE=0 HK_LOG=debug hk validate
```

## A tool's gone missing {#a-command-is-missing}

The standing crew don't bring their own tools: builtins do not install linters. Check that the named executable is aboard the environment that runs hk.

If it works in yer terminal but fails from Git or an editor, use the recommended `hk install --global --mise` on Git 2.54+. mise must be on `PATH` while ye install; the global launcher writes down its path, so Git need not find mise at runtime. For a repository-scoped installation on any supported Git version, use `hk install --mise`; this local launcher needs mise on Git's `PATH` at runtime. For language package dependencies, expose the package's executable directory. See [mise, the quartermaster](/mise_integration).

## The hook stays silent, or pipes twice {#a-hook-does-not-fire-or-fires-twice}

Inspect how the hooks are rigged without re-rigging them:

```sh
git config --show-origin --get-regexp '^hook\.hk-'
git config --show-origin --get core.hooksPath
```

These commands may exit nonzero if no matching setting exists. On an older Git or with `--legacy`, inspect the hook scripts that apply as well, matey.

Use `hk run pre-commit --plan` to confirm the configured hook can be loaded. Run `hk install` to re-rig (refresh) a local installation, or `hk install --global` for a global one.

A normal local install spots global hk hooks already rigged and won't duplicate them. A local install ye explicitly force alongside global hooks can make the bosun pipe twice: duplicate runs. See [rigging the hooks, installation](/getting_started#install-hooks).

## The hook mends more than ye expected {#a-hook-changes-more-than-expected}

Compare `git diff` with `git diff --cached`. A staged path can still carry unstaged edits, and sailmakers (formatters) mend whole files.

Use `stash = "git"` to stow the unstaged work in the hold, isolating the staged content before a pre-commit fixer runs. Use `stage = false` with `fail_on_fix = true` to look over the fixes before ye commit. See [hooks and stowing the hold](/hooks).

## A slow passage {#a-run-is-slow}

Have hk write a timing report:

```sh
HK_TIMING_JSON=hk-timing.json hk check --all
```

The report gives the wall time for the whole run and for each step:

```json
{
  "total": { "wall_time_ms": 12456 },
  "steps": {
    "lint": { "wall_time_ms": 4321, "profiles": ["slow"] },
    "format": { "wall_time_ms": 2100 }
  }
}
```

A step's time merges the overlapping intervals within that step. Different steps can haul at once, so their durations don't sum to the total run time.

Look for costly lookouts (expensive linters), `exclusive` settings ye don't need, file patterns cast too wide, and dependencies that make the hands haul one after another. If the linters already parallelize internally, compare with fewer jobs. On a very large worktree, finding the untracked files can cost dear too; see [`HK_STASH_UNTRACKED`](/environment_variables#hk-stash-untracked).

## How much goes in the ship's log {#log-levels}

| Level   | What gets written                        |
| ------- | ---------------------------------------- |
| `error` | Errors                                   |
| `warn`  | Warnings and errors                      |
| `info`  | Informational messages; the default      |
| `debug` | File selection and execution details     |
| `trace` | Finer detail of hk's inner workings      |

```sh
hk check -v          # Debug logging
hk check -vv         # Trace logging
HK_LOG=debug hk check
```

Use `--quiet` to turn the output down, or `--silent` to hush it entirely. Summaries of failed steps stay useful in the harbour-master's plain-text CI output; `HK_SUMMARY_TEXT=1` asks for summaries of the successful steps too.

### Where the log is kept {#log-files}

The ship's log goes to `$HK_STATE_DIR/hk.log` unless ye say otherwise. On Linux, that's typically `~/.local/state/hk/hk.log`. Ye can override the file path and the level independently:

```sh
HK_LOG_FILE=/tmp/hk-debug.log HK_LOG_FILE_LEVEL=trace hk check
```

## Tracing every leg {#tracing}

Tracing records spans and timing on top of the ordinary log:

```sh
hk check --trace
HK_TRACE=1 hk check
HK_TRACE=json hk check > trace.jsonl
```

Text tracing goes to standard error. JSON tracing writes its events to standard output. Commands that print their own standard output may share that stream, so inspect it before ye treat the entire file as JSONL.

## Sing out a bug {#report-a-bug}

When ye sing out "Bug, ho!", include the hk version, operating system, Git version, the relevant configuration, the exact command, and the first substantive error from the verbose output. For file-selection or stashing trouble, describe which files or hunks were staged. Before posting logs to [GitHub issues](https://github.com/jdx/hk/issues), review them for private paths, command output, and environment values.
