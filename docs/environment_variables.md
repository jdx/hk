---
outline: "deep"
---

# Environment Variables

Environment variables can be used to configure hk.

Most of these map to settings that can also be configured via CLI flags, git config, `hk.pkl`, or user config. See the
[Settings Reference](/configuration#settings-reference) for every setting's available sources and precedence. Variables are listed here in alphabetical order.

## `HK_CACHE`

Type: `bool`
Default: `true` (release builds), `false` (debug builds)

Controls whether hk caches data such as parsed configuration files. Set to `0` or `false` to disable caching.

## `HK_CACHE_DIR`

Type: `path`
Default: `~/.cache/hk`

The cache directory to use.

## `HK_CHECK`

Type: `bool`
Default: `false`

Forces hk to run only check commands (read-only) instead of fix commands. This is the opposite of `HK_FIX` and is equivalent to passing `--check`.

Useful for CI environments where you want to verify code quality without making changes.

## `HK_CHECK_FIRST`

Type: `bool`
Default: `true`

If `true`, when multiple steps match the same file, hk runs their check commands first and only runs a step's fix command if its check fails.

This lets hk parallelize as much as possible. Any number of check commands can run against the same file at once without interfering with each other, but two fix commands cannot safely write to the same file. So hk optimistically runs the check commands in parallel and then, for any that fail, runs the fix commands, serialized where they share files.

If disabled, hk skips the check pass and runs the overlapping fix commands in series.

## `HK_CONFIG_DIR`

Type: `path`
Default: `$XDG_CONFIG_HOME/hk` (usually `~/.config/hk`)

The directory hk uses for user configuration such as `~/.config/hk/config.pkl`.

## `HK_DISPLAY_SKIP_REASONS`

Type: `string[]` (comma-separated list)
Default: `profile-not-enabled`

Controls which skip reasons are displayed when steps are skipped. A step skipped for a reason not in this list is skipped silently.

Available reasons:

- `profile-not-enabled`: a profile the step requires is not enabled (default)
- `profile-explicitly-disabled`: a profile the step requires was disabled with `!profile`
- `disabled-by-config`: the step is listed in `skip_steps` in `hk.pkl`, user config, or git config
- `disabled-by-env`: the step is listed in `HK_SKIP_STEPS`
- `disabled-by-cli`: the step was skipped with `--skip-step`
- `no-command-for-run-type`: the step has no command for the current mode (for example no `fix` command during `hk fix`)
- `no-files-to-process`: no files matched the step's globs
- `condition-false`: the step's `condition` evaluated to false
- `missing-required-env`: an environment variable listed in the step's `required` setting is unset

Example: `HK_DISPLAY_SKIP_REASONS=profile-not-enabled,no-files-to-process`.

## `HK_EXCLUDE`

Type: `string[]` (comma-separated list)
Default: `(empty)`

A comma-separated list of glob patterns to exclude from processing. These patterns are unioned with exclude patterns from other configuration sources (git config, user config, project config). Supports both directory names and glob patterns.

Examples:
```bash
# Exclude specific directories
HK_EXCLUDE=node_modules,dist

# Exclude using glob patterns
HK_EXCLUDE="**/*.min.js,**/*.map"
```

## `HK_FAIL_FAST`

Type: `bool`
Default: `true`

If `true`, hk will abort running steps after the first one fails.

## `HK_FILE`

Type: `string`
Default: `hk.pkl`

The config file to use. Setting this skips the normal [config file search](/configuration#config-file-paths).

## `HK_FIX`

Type: `bool`
Default: `true`

If set to `false`, hk runs check commands instead of fix commands, the same as passing `--check`. `HK_CHECK=1` has the same effect.

## `HK_HIDE_WARNINGS`

Type: `string[]` (comma-separated list)
Default: `(empty)`

A comma-separated list of warning tags to suppress. This allows you to hide specific warning messages that you don't want to see.

Available warning tags:

- `missing-profiles`: Suppresses warnings about steps being skipped due to missing profiles

Example usage:

```bash
HK_HIDE_WARNINGS=missing-profiles hk check
```

## `HK_HIDE_WHEN_DONE`

Type: `bool`
Default: `false`

If set to `true`, hk will hide the progress output when the hook finishes if there are no errors.

## `HK_JOBS`

Type: `usize`
Default: `(number of cores)`

The number of jobs to run in parallel. `HK_JOB` is accepted as an alias.

## `HK_JSON`

Type: `bool`
Default: `false`

Enables JSON output format for structured data, equivalent to passing `--json`. Useful for integration with other tools or for programmatic processing of results.

Example: `hk check --json | jq '.steps[] | select(.failed)'`

## `HK_LIBGIT2`

Type: `bool`
Default: `true`

If set to `false`, hk shells out to the `git` command instead of using libgit2. This may perform better
in some cases, such as repositories that use `fsmonitor` to watch for changes.

## `HK_LOG`

Type: `off` | `error` | `warn` | `info` | `debug` | `trace`
Default: `info`

The log level to use. `HK_LOG_LEVEL` is accepted as an alias.

## `HK_LOG_FILE`

Type: `path`
Default: `~/.local/state/hk/hk.log`

The log file to use.

## `HK_LOG_FILE_LEVEL`

Type: `off` | `error` | `warn` | `info` | `debug` | `trace`
Default: `HK_LOG`

The log level to use for the log file.

## `HK_MISE`

Type: `bool`
Default: `false`

If set to `true`:

- `hk install` installs hooks that run hk through `mise x`, so mise-managed tools are available without activating mise in the shell
- `hk init` also creates a `mise.toml` file with hk configured
- Steps with a `dir` get the mise environment for that directory (`mise env`, cached per directory), so tools and env vars from the directory's mise config are available. See [mise integration](/mise_integration#per-directory-environments-monorepos)

## `HK_OUTPUT_FILE`

Type: `path`
Default: `~/.local/state/hk/output.log`

The file where hk writes the complete output of a failed command. An empty value uses the default location.


## `HK_PKL_BACKEND`

Type: `pkl` | `pklr`
Default: `pklr`

Selects the evaluator used to read `hk.pkl`. Set to `pkl` to use the pkl CLI instead of the built-in pklr evaluator.

## `HK_PKL_CA_CERTIFICATES`

Type: `path`

A path to a CA certificates file to provide `pkl`'s `--ca-certificates` flag when invoking `pkl`.

This is useful in corporate environments with SSL-intercepting proxies where pkl needs to trust custom CA certificates to download packages.

This variable is read directly from the environment before pkl is invoked, so it cannot be configured in `hk.pkl`.

## `HK_PKL_CACHE_DIR`

Type: `path`
Default: the platform cache directory with `pklr` appended (`~/.cache/pklr` on Linux, `~/Library/Caches/pklr` on macOS, and `%LOCALAPPDATA%\pklr` on Windows). Falls back to `~/.cache/pklr` when the platform cache directory is unavailable.

The directory used by the built-in pklr evaluator to persist downloaded Pkl packages. Packages in this cache can be reused after hk's resolved configuration cache is invalidated, including in offline mode.

This variable is read directly from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`.

## `HK_PKL_EMBEDDED`

Type: `bool`
Default: `true`

hk embeds the Pkl package for its own version and seeds `HK_PKL_CACHE_DIR` with it before evaluating `hk.pkl`. A config pinning the running hk version therefore evaluates without a network request, even on a cold cache. A config pinning any other version is downloaded as usual.

Set to `0` to disable seeding. The package is then resolved from `HK_PKL_CACHE_DIR`, falling back to the network unless `HK_PKL_OFFLINE` is set, in which case a package missing from the cache fails.

This variable is read directly from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`.

## `HK_PKL_HTTP_REWRITE`

Type: `string`

A value to provide `pkl`'s `--http-rewrite` flag when invoking `pkl`, in the form `http(s)://<FROM>/=http(s)://<TO>/`.

This variable is read directly from the environment before pkl is invoked, so it cannot be configured in `hk.pkl`.

## `HK_PKL_OFFLINE`

Type: `bool`
Default: `false`

Disables network access in the built-in pklr evaluator. Package imports already present in `HK_PKL_CACHE_DIR`, along with the package embedded for the running version (see `HK_PKL_EMBEDDED`), remain available; a missing package fails immediately with its URL and cache location.

This variable is read directly from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`.

## `HK_PROFILE`

Type: `string[]` (comma-separated list)

The profile(s) to enable. Prefix a profile with `!` to explicitly disable it. `HK_PROFILES` is accepted as an alias.

Example usage:

- `HK_PROFILE=ci` - Enable the CI profile
- `HK_PROFILE=slow,ci` - Enable multiple profiles

## `HK_SKIP_HOOK`

Type: `string[]` (comma-separated list)
Default: `(empty)`

A comma-separated list of hook names to skip entirely.
For example: `HK_SKIP_HOOK=pre-commit,pre-push` skips those hooks completely. `HK_SKIP_HOOKS` is accepted as an alias.

This is useful when you want to temporarily disable certain hooks while still keeping them configured in your `hk.pkl` file.
Unlike `HK_SKIP_STEPS` which skips individual steps, this skips the entire hook and all its steps.

This setting can also be configured via:
- Git config: `git config hk.skipHook "pre-commit"`
- User config (`~/.config/hk/config.pkl`): `skip_hooks = List("pre-commit")`

All skip configurations from different sources are unioned together.

## `HK_SKIP_STEPS`

Type: `string[]` (comma-separated list)

A comma-separated list of step names to skip in every hook.
For example: `HK_SKIP_STEPS=lint,test` skips any steps named "lint" or "test". `HK_SKIP_STEP` is accepted as an alias.

This setting can also be configured via:
- Git config: `git config hk.skipSteps "step1,step2"`
- User config (`~/.config/hk/config.pkl`): `skip_steps = List("step1", "step2")`

All skip configurations from different sources are unioned together.

## `HK_STAGE`

Type: `bool`

When set, overrides the [hook's `stage` key](/configuration#hooks-hook-stage-boolean), which controls whether hk automatically stages files modified by fix commands.

This is useful when you want to manually review changes made by auto-fixers before including them in your commit.

## `HK_STASH`

Type: `git` | `patch-file` | `none`
Default: `none`

Overrides the [hook-level `stash` setting](/configuration), which defaults to `none`.

- `git`: Use `git stash` to stash unstaged changes before running hooks.
- `patch-file`: Currently an alias of the `git` behavior.
- `none`: Do not stash unstaged changes before running hooks. Fastest option, but fix steps may modify unstaged changes if they are in the same file as staged changes.

In `hk.pkl`, the hook-level `stash` key also accepts booleans: `true` is an alias of `"git"` and `false` is an alias of `"none"`.

## `HK_STASH_BACKUP_COUNT`

Type: `usize`
Default: `20`

Number of backup patch files to keep per repository when stashing. Each time hk stashes changes, it writes a backup patch file to `$HK_STATE_DIR/patches/`; the oldest backups beyond this count are deleted automatically.

Set to `0` to disable patch backup creation entirely.

## `HK_STASH_UNTRACKED`

Type: `bool`
Default: `true`

If set to `true`, hk will stash untracked files when stashing before running hooks.

When set to `false`, hk also skips the untracked-file scan entirely (`git status --untracked-files=no`). This is the recommended setting when `GIT_WORK_TREE` points at a very large directory such as `$HOME` (e.g. a YADM dotfiles repo), where scanning for untracked files can take tens of seconds. Untracked files will not appear in reports or `hk check --all` results in this mode.

## `HK_STATE_DIR`

Type: `path`
Default: `~/.local/state/hk`

The state directory to use.

## `HK_SUMMARY_TEXT`

Type: `bool`
Default: `false`

Controls whether per-step output summaries are printed in plain text mode. By default, text mode only prints summaries for **failed** steps, so CI logs always include the full diagnostics for a failure. Successful steps stream their output during execution, so a trailing summary would only duplicate it. Set this to `true` to print a summary for every step in text mode.

Example:

```bash
HK_SUMMARY_TEXT=1 hk check
```

## `HK_TERMINAL_PROGRESS`

Type: `bool`
Default: `true`

Enables or disables reporting progress via OSC sequences to compatible terminals.

## `HK_TIMING_JSON`

Type: `path`

If set to a file path, hk will write a JSON timing report at the end of a run. The report includes total wall time and per-step wall time, with overlapping intervals merged so time isn't double-counted across parallel step parts.

The `steps` field is an object mapping step names to an object with:

- `wall_time_ms`: merged wall time in milliseconds
- `profiles` (optional): the list of profiles required for that step. If there are no profiles, this field is omitted.

Example usage:

```bash
HK_TIMING_JSON=/tmp/hk-timing.json hk check
```

Additionally, when a hook-level `report` command is configured in `hk.pkl`, hk will set `HK_REPORT_JSON` to the same timing JSON content (in-memory) and execute the command after the hook finishes. This enables custom scripts to post-process or upload timing data without reading a file.

Example output shape:

```json
{
  "total": { "wall_time_ms": 12456 },
  "steps": {
    "lint": { "wall_time_ms": 4321, "profiles": ["ci", "fast"] },
    "fmt": { "wall_time_ms": 2100 }
  }
}
```

## `HK_TRACE`

Type: `off` | `text` | `json` | `1` | `true`
Default: `off`

Enables tracing spans and performance diagnostics for detailed execution analysis.

- `off`: No tracing (default)
- `text` (or the aliases `1` / `true`): Human-readable trace output
- `json`: Machine-readable JSON trace output

Example: `HK_TRACE=text hk check` to see detailed execution traces.

## `HK_WALK_IGNORE`

Type: `bool`
Default: `true`

Controls whether hk respects `.gitignore` and other ignore files when walking directories.

When enabled (default), hk will skip files matching patterns in `.gitignore`, `.ignore`, and other standard ignore files when discovering files for linting. This improves performance by not processing generated files, build artifacts, or vendored dependencies.

When disabled, all files are included regardless of ignore patterns.

Example: `HK_WALK_IGNORE=0 hk check --all` to include all files.

## `HK_WARNINGS`

Type: `string[]` (comma-separated list)
Default: `(empty)`

Warning tags to enable. This can also be configured in `hk.pkl` or user config:

```pkl
warnings = List("missing-profiles")
```
