---
description: Diagnose skipped steps, configuration problems, missing tools, hook failures, and slow runs.
---

# Troubleshooting

Start with a plan and verbose output. They usually show whether the problem is file selection, configuration, or a linter command.

```sh
hk check --plan
hk check -v
```

## A step does not run

Ask hk to explain the step:

```sh
hk check --why eslint
hk check --all --why eslint
```

Replace `eslint` with the name in your configuration. Check its file patterns, exclusions, required environment variables, conditions, and profiles.

`hk check` normally selects modified files. `--all` expands the selection. A step with a `slow` profile needs `--slow` or `--profile slow`; a step with multiple positive profiles requires all of them.

To inspect the plan as JSON:

```sh
hk check --all --plan --json
```

Plans do not run linter commands. Configuration evaluation and condition evaluation may still inspect the environment.

## Configuration is not what you expect

```sh
hk validate -v
hk config dump
hk config get skip_steps
hk config explain jobs
```

`hk validate` checks the Pkl configuration. `hk config dump` shows effective runtime settings, rather than the complete hook and step definitions. `hk config explain` helps identify overrides.

Check for `hk.local.pkl`, a user config, Git settings, and `HK_*` environment variables. See [configuration precedence](/configuration#configuration-precedence).

For evaluator or cache issues:

```sh
HK_CACHE=0 hk validate
HK_PKL_BACKEND=pkl hk validate
```

The second command requires the Pkl CLI. See [Pkl evaluators](/pkl_introduction#evaluators).

## A command is missing

Builtins do not install linters. Check that the named executable is available in the environment running hk.

If it works in your terminal but fails from Git or an editor, install the launcher with `hk install --mise` when using mise. Git must still be able to find mise itself. For language package dependencies, expose the package’s executable directory. See [mise integration](/mise_integration).

## A hook does not fire, or fires twice

Inspect installation without rewriting it:

```sh
git config --show-origin --get-regexp '^hook\.hk-'
git config --show-origin --get core.hooksPath
```

These commands may exit nonzero if no matching setting exists. On older Git or with `--legacy`, inspect the applicable hook scripts too.

Use `hk run pre-commit --plan` to confirm the configured hook can be loaded. Run `hk install` to refresh a local installation, or `hk install --global` for a global one.

A normal local install detects existing global hk hooks and avoids duplicating them. An explicitly forced local install alongside global hooks can cause duplicate runs. See [installation](/getting_started#install-hooks).

## A hook changes more than expected

Compare `git diff` and `git diff --cached`. A staged path can contain unstaged edits, and formatters work on whole files.

Use `stash = "git"` to isolate staged content before a pre-commit fixer runs. Use `stage = false` with `fail_on_fix = true` to review fixes before committing. See [hooks and stashing](/hooks).

## A run is slow

Write a timing report:

```sh
HK_TIMING_JSON=hk-timing.json hk check --all
```

The report contains total and per-step wall time:

```json
{
  "total": { "wall_time_ms": 12456 },
  "steps": {
    "lint": { "wall_time_ms": 4321, "profiles": ["slow"] },
    "format": { "wall_time_ms": 2100 }
  }
}
```

Step time merges overlapping intervals within that step. Different steps can overlap, so their durations do not sum to total run time.

Look for expensive linters, unnecessary `exclusive` settings, broad file patterns, and dependencies that serialize work. Compare with fewer jobs if the linters already parallelize internally. For very large worktrees, untracked-file discovery can also be costly; see [`HK_STASH_UNTRACKED`](/environment_variables#hk-stash-untracked).

## Log levels

| Level   | Output                               |
| ------- | ------------------------------------ |
| `error` | Errors                               |
| `warn`  | Warnings and errors                  |
| `info`  | Informational messages; the default  |
| `debug` | File selection and execution details |
| `trace` | More detailed internal operations    |

```sh
hk check -v          # Debug logging
hk check -vv         # Trace logging
HK_LOG=debug hk check
```

Use `--quiet` to reduce output or `--silent` to suppress it. Failed-step summaries remain useful in plain-text CI output; `HK_SUMMARY_TEXT=1` also requests summaries for successful steps.

### Log files

Logs go to `$HK_STATE_DIR/hk.log` by default. On Linux, this is typically `~/.local/state/hk/hk.log`. Override the file path and level independently:

```sh
HK_LOG_FILE=/tmp/hk-debug.log HK_LOG_FILE_LEVEL=trace hk check
```

## Tracing

Tracing records spans and timing in addition to ordinary logs:

```sh
hk check --trace
HK_TRACE=1 hk check
HK_TRACE=json hk check > trace.jsonl
```

Text tracing goes to standard error. JSON tracing writes events to standard output. Commands that print their own standard output may share that stream, so inspect it before treating the entire file as JSONL.

## Report a bug

Include the hk version, operating system, Git version, relevant configuration, exact command, and the first substantive error from verbose output. For file-selection or stashing issues, describe which files or hunks were staged. Review logs for private paths, command output, and environment values before posting them to [GitHub issues](https://github.com/jdx/hk/issues).
