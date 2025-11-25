### `all`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--all`

Enables running all available steps, including those that might be disabled by default or require specific profiles.

When enabled, hk will run all configured steps regardless of their profile requirements or other filtering criteria.
This is useful for comprehensive checking or when you want to run every possible linting/formatting task.

Example: `hk check --all` to run all available checks.

### `cache-dir`

- Type: `path`
- Sources:
  - ENV: `HK_CACHE_DIR`

Directory where hk stores cache files for improved performance.

Default location: `~/.cache/hk`

Cache includes tool binary locations, parsed configurations, and other performance optimizations.

### `check`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--check`, `-c`
  - ENV: `HK_CHECK`
  - Git: `hk.check`

Forces hk to run only check commands (read-only) instead of fix commands.

This is the opposite of the `fix` setting. When enabled, hk will report issues without attempting to fix them.

Useful for CI environments where you want to verify code quality without making changes.

### `check-first`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_CHECK_FIRST`
  - Git: `hk.checkFirst`

If enabled, hk will run check commands first, then run fix commands only if the check fails when there are multiple linters with the same file in matching glob patterns.

The reason for this optimization is to maximize parallelization. We can have multiple check commands running in parallel against the same file without interference, but we can't have 2 fix commands potentially writing to the same file simultaneously.

If disabled, hk will use simpler logic that just runs fix commands in series in this situation.

### `display-skip-reasons`

- Type: `list<string>`
- Default: `["profile-not-enabled"]`
- Sources:
  - ENV: `HK_DISPLAY_SKIP_REASONS`
  - Git: `hk.displaySkipReasons`
  - Pkl: `display_skip_reasons`

Controls which skip reasons are displayed when steps are skipped.

Available options:
- `all`: Show all skip reasons
- `none`: Hide all skip reasons
- `disabled-by-config`: Show when steps are skipped due to configuration
- `profile-not-enabled`: Show when steps are skipped due to missing profiles (default)

Example: `HK_DISPLAY_SKIP_REASONS=all` to see all skip reasons.

### `exclude`

- Type: `list<string>`
- Sources:
  - CLI: `--exclude`, `-e`
  - ENV: `HK_EXCLUDE`
  - Git: `hk.exclude`
  - Pkl: `exclude`

Glob patterns to exclude from processing. These patterns are **unioned** with exclude patterns from other configuration sources (git config, user config, project config). Supports both directory names and glob patterns.

Examples:
- Exclude specific directories: `node_modules,dist`
- Exclude using glob patterns: `**/*.min.js,**/*.map`

All exclude patterns from different sources are combined.

### `fail-fast`

- Type: `bool`
- Default: `true`
- Sources:
  - CLI: `--fail-fast`, `--no-fail-fast`
  - ENV: `HK_FAIL_FAST`
  - Git: `hk.failFast`
  - Pkl: `fail_fast`

Controls whether hk aborts running steps after the first one fails.

When enabled (default), hk will stop execution immediately when a step fails, providing quicker feedback.
When disabled, hk will continue running all steps even if some fail, useful for seeing all issues at once.

Can be toggled with `--fail-fast` / `--no-fail-fast` CLI flags.

### `fix`

- Type: `bool`
- Default: `true`
- Sources:
  - CLI: `--fix`, `-f`
  - ENV: `HK_FIX`
  - Git: `hk.fix`

Controls whether hk runs fix commands (that modify files) or check commands (read-only).

When enabled (default), runs fix commands to automatically resolve issues.
When disabled, only runs check commands to report issues without making changes.

Can be toggled with `--fix` / `--check` CLI flags.

### `hide-warnings`

- Type: `list<string>`
- Sources:
  - ENV: `HK_HIDE_WARNINGS`
  - Git: `hk.hideWarnings`
  - Pkl: `hide_warnings`

Warning tags to suppress. Allows hiding specific warning messages that you don't want to see.

Available warning tags:
- `missing-profiles`: Suppresses warnings about steps being skipped due to missing profiles

Example: `HK_HIDE_WARNINGS=missing-profiles`

All hide configurations from different sources are **unioned** together.

### `hide-when-done`

- Type: `bool`
- Default: `false`
- Sources:
  - ENV: `HK_HIDE_WHEN_DONE`

Controls whether hk hides the progress output when the hook finishes successfully.

When enabled, successful runs will clear their output to reduce visual clutter.
Failed runs will always show their output regardless of this setting.

### `hkrc`

- Type: `path`
- Default: `".hkrc.pkl"`
- Sources:
  - CLI: `--hkrc`

Path to the user configuration file.

Default: `.hkrc.pkl` in the current directory or parent directories.

This file can override project-level settings and is useful for personal preferences.

### `jobs`

- Type: `usize`
- Default: `0`
- Sources:
  - CLI: `--jobs`, `-j`
  - ENV: `HK_JOBS`, `HK_JOB`
  - Git: `hk.jobs`
  - Pkl: `jobs`

The number of parallel processes that hk will use to execute steps concurrently. This affects performance by controlling how many linting/formatting tasks can run simultaneously.

Set to `0` (default) to auto-detect based on CPU cores.

Example usage:
- `hk check --jobs 4` - Run with 4 parallel jobs
- `HK_JOBS=8 hk fix` - Set via environment variable

### `json`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--json`
  - ENV: `HK_JSON`
  - Git: `hk.json`

Enables JSON output format for structured data.

When enabled, hk outputs machine-readable JSON instead of human-readable text.
Useful for integration with other tools or for programmatic processing of results.

Example: `hk check --json | jq '.steps[] | select(.failed)'`

### `libgit2`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_LIBGIT2`

Controls whether hk uses libgit2 (a Git library) or shells out to git CLI commands.

When enabled (default), uses libgit2 for better performance in most cases.
When disabled, uses git CLI commands which may provide better performance in some cases such as when using `fsmonitor` to watch for changes.

### `log-file`

- Type: `path`
- Sources:
  - ENV: `HK_LOG_FILE`

Path to the log file where hk writes detailed execution logs.

Default location: `~/.local/state/hk/hk.log`

Useful for debugging issues or keeping an audit trail of hook executions.

### `log-file-level`

- Type: `enum`
- Default: `"info"`
- Sources:
  - ENV: `HK_LOG_FILE_LEVEL`

Controls the verbosity of file logging output.

Uses the same levels as `log_level` but specifically for the log file.
Defaults to the same level as `log_level` if not specified.

This allows you to have different verbosity levels for console and file output.

### `log-level`

- Type: `enum`
- Default: `"info"`
- Sources:
  - ENV: `HK_LOG`, `HK_LOG_LEVEL`

Controls the verbosity of console output.

Available levels (from least to most verbose):
- `off`: No logging
- `error`: Only errors
- `warn`: Errors and warnings
- `info`: Normal output (default)
- `debug`: Detailed debugging information
- `trace`: Very detailed trace information

Example: `HK_LOG_LEVEL=debug hk check`

### `mise`

- Type: `bool`
- Default: `false`
- Sources:
  - ENV: `HK_MISE`

Enables deep integration with [mise](https://mise.jdx.dev) for tool management.

When enabled:
- `hk install` will use `mise x` to execute hooks, ensuring mise tools are available without activation
- `hk init` will create a `mise.toml` file with hk configured
- Tool discovery will use mise shims automatically

### `no-progress`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--no-progress`

Disables progress bars and real-time status updates.

When enabled, hk will use simpler text output instead of dynamic progress indicators.
Useful for CI environments or when output is being logged to a file.

### `profiles`

- Type: `list<string>`
- Sources:
  - CLI: `--profile`, `-p`
  - ENV: `HK_PROFILE`, `HK_PROFILES`
  - Git: `hk.profile`
  - Pkl: `profiles`

Profiles to enable or disable. Profiles allow you to group steps that should run only in certain contexts (e.g., CI, slow tests).

Prefix with `!` to explicitly disable a profile.

Example usage:
- `HK_PROFILE=ci` - Enable the CI profile
- `HK_PROFILE=slow,ci` - Enable multiple profiles
- `--profile=!slow` - Explicitly disable the slow profile

### `quiet`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--quiet`, `-q`

Suppresses non-essential output.

When enabled, only errors and critical information will be displayed.
Useful for scripting or when you only care about the exit code.

### `silent`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--silent`

Completely suppresses all output, including errors.

More extreme than `quiet` - absolutely no output will be displayed.
Useful when only the exit code matters.

### `skip-hooks`

- Type: `list<string>`
- Sources:
  - ENV: `HK_SKIP_HOOK`, `HK_SKIP_HOOKS`
  - Git: `hk.skipHooks`, `hk.skipHook`
  - Pkl: `skip_hooks`

A list of hook names to skip entirely. This allows you to disable specific git hooks from running.

For example: `HK_SKIP_HOOK=pre-commit,pre-push` would skip running those hooks completely.

This is useful when you want to temporarily disable certain hooks while still keeping them configured in your `hk.pkl` file.
Unlike `skip_steps` which skips individual steps, this skips the entire hook and all its steps.

This setting can also be configured via:
- Git config: `git config hk.skipHook "pre-commit"`
- User config (`.hkrc.pkl`): `skip_hooks = List("pre-commit")`

**All skip configurations from different sources are unioned together.**

### `skip-steps`

- Type: `list<string>`
- Sources:
  - CLI: `--skip-step`
  - ENV: `HK_SKIP_STEPS`, `HK_SKIP_STEP`
  - Git: `hk.skipSteps`, `hk.skipStep`
  - Pkl: `skip_steps`

A list of step names to skip when running hooks. This allows you to bypass specific linting or formatting tasks.

For example: `HK_SKIP_STEPS=lint,test` would skip any steps named "lint" or "test".

This setting can also be configured via:
- Git config: `git config hk.skipSteps "step1,step2"`
- User config (`.hkrc.pkl`): `skip_steps = List("step1", "step2")`

**All skip configurations from different sources are unioned together.**

### `slow`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--slow`, `-s`

Enables the "slow" profile for running additional checks that may take longer.

This is a convenience flag equivalent to `--profile=slow`.

Useful for thorough checking in CI or before major releases.

### `stage`

- Type: `bool`
- Sources:
  - CLI: `--stage`, `--no-stage`
  - ENV: `HK_STAGE`
  - Git: `hk.stage`
  - Pkl: `stage`

When specified, overrides the [hook's `stage` key](https://hk.jdx.dev/configuration.html#hooks-hook-stage-boolean).

This is useful when you want to manually review changes made by auto-fixers before including them in your commit.

### `stash`

- Type: `enum`
- Sources:
  - CLI: `--stash`
  - ENV: `HK_STASH`
  - Git: `hk.stash`

Strategy for temporarily saving unstaged changes before running hooks that might modify files. This prevents conflicts between your working directory changes and automated fixes.

Available strategies:
- `git`: Use `git stash` to stash changes
- `patch-file`: Use hk-generated patch files (typically faster, avoids "index is locked" errors)
- `none`: No stashing (fastest, but may cause staging conflicts if fixes modify unstaged changes in the same file)

### `stash-backup-count`

- Type: `usize`
- Default: `20`
- Sources:
  - ENV: `HK_STASH_BACKUP_COUNT`
  - Git: `hk.stashBackupCount`
  - Pkl: `stash_backup_count`

Number of backup patch files to keep per repository when using git stash.

Each time git stash is used, hk creates a backup patch file in
$HK_STATE_DIR/patches/. This setting controls how many of these
backups are retained per repository (oldest are automatically deleted).

Set to 0 to disable patch backup creation entirely.

Default: 20

### `stash-untracked`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_STASH_UNTRACKED`
  - Git: `hk.stashUntracked`

Controls whether untracked files are included when stashing before running hooks.

When enabled (default), untracked files will be temporarily stashed along with tracked changes.
This ensures a clean working directory for hook execution.

### `state-dir`

- Type: `path`
- Sources:
  - ENV: `HK_STATE_DIR`

Directory where hk stores persistent state files.

Default location: `~/.local/state/hk`

Includes logs, temporary patch files for stashing, and other state information.

### `summary-text`

- Type: `bool`
- Default: `false`
- Sources:
  - ENV: `HK_SUMMARY_TEXT`

Controls whether per-step output summaries are printed in plain text mode.

By default, summaries are only shown when hk is rendering progress bars (non-text mode).
Set to `true` to force summaries to appear in text mode, useful for CI environments.

Example: `HK_SUMMARY_TEXT=1 hk check`

### `terminal-progress`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_TERMINAL_PROGRESS`
  - Git: `hk.terminalProgress`
  - Pkl: `terminal_progress`

Enables or disables reporting progress via OSC sequences to compatible terminals.

### `timing-json`

- Type: `path`
- Sources:
  - ENV: `HK_TIMING_JSON`

Path to write a JSON timing report after a hook finishes. The report includes total wall time and per-step wall time, with overlapping intervals merged so time isn't double-counted across parallel step parts.

The `steps` field maps step names to objects containing:
- `wall_time_ms`: merged wall time in milliseconds
- `profiles` (optional): list of profiles required for that step

Example usage: `HK_TIMING_JSON=/tmp/hk-timing.json hk check`

Example output:
```json
{
  "total": { "wall_time_ms": 12456 },
  "steps": {
    "lint": { "wall_time_ms": 4321, "profiles": ["ci", "fast"] },
    "fmt": { "wall_time_ms": 2100 }
  }
}
```

When a hook-level `report` command is configured in `hk.pkl`, hk will set `HK_REPORT_JSON` to the same timing JSON content and execute the command after the hook finishes.

### `trace`

- Type: `enum`
- Default: `"off"`
- Sources:
  - CLI: `--trace`
  - ENV: `HK_TRACE`
  - Git: `hk.trace`

Enables tracing spans and performance diagnostics for detailed execution analysis.

Available formats:
- `off`: No tracing (default)
- `text`: Human-readable trace output
- `json`: Machine-readable JSON trace output
- `1` or `true`: Enable text tracing (aliases)

Useful for debugging performance issues or understanding execution flow.

Example: `HK_TRACE=text hk check` to see detailed execution traces.

### `verbose`

- Type: `u8`
- Default: `0`
- Sources:
  - CLI: `--verbose`, `-v`

Controls the verbosity of output.

Can be specified multiple times to increase verbosity:
- `-v`: Basic verbose output
- `-vv`: More detailed output
- `-vvv`: Very detailed output

Example: `hk check -vv` for detailed debugging output.

### `warnings`

- Type: `list<string>`
- Sources:
  - ENV: `HK_WARNINGS`
  - Git: `hk.warnings`
  - Pkl: `warnings`

Warning tags to enable or show. Controls which warning messages are displayed during execution.
