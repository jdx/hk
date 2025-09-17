# HK Configuration Reference

This document is auto-generated from `settings.toml`.

## `all`

Run on all files instead of just staged files.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### CLI Flags
- `--all`
- `-a`

## `cache_dir`

Directory for cache files.

- **Type**: `path`
- **Default**: `String("")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_CACHE_DIR`

## `check`

Run check command instead of fix command.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_CHECK`

### Git Config Keys
- `hk.check`

### CLI Flags
- `--check`
- `-c`

## `check_first`

Run check before fix.

- **Type**: `bool`
- **Default**: `Boolean(true)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_CHECK_FIRST`

### Git Config Keys
- `hk.checkFirst`

## `display_skip_reasons`

Which skip reasons to display. Options: all, none, disabled-by-config, profile-not-enabled.

- **Type**: `list<string>`
- **Default**: `Array([String("profile-not-enabled")])`
- **Merge Policy**: `replace`

### PKL Sources
- `display_skip_reasons`

### Environment Variables
- `HK_DISPLAY_SKIP_REASONS`

### Git Config Keys
- `hk.displaySkipReasons`

## `exclude`

Glob patterns to exclude from processing (union across sources).

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `union`

### PKL Sources
- `defaults.exclude`

### Environment Variables
- `HK_EXCLUDE`

### Git Config Keys
- `hk.exclude`
- `hk.excludeGlob`

### CLI Flags
- `--exclude`
- `--exclude-glob`
- `-e`

## `fail_fast`

Abort execution on first failure.

- **Type**: `bool`
- **Default**: `Boolean(true)`
- **Merge Policy**: `replace`

### PKL Sources
- `fail_fast`
- `defaults.fail_fast`

### Environment Variables
- `HK_FAIL_FAST`

### Git Config Keys
- `hk.failFast`

### CLI Flags
- `--fail-fast`
- `--no-fail-fast`

## `file`

Default configuration file name.

- **Type**: `string`
- **Default**: `String("hk.pkl")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_FILE`

## `files`

Specific files to run on.

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `replace`

## `fix`

Run fix command instead of check command.

- **Type**: `bool`
- **Default**: `Boolean(true)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_FIX`

### Git Config Keys
- `hk.fix`

### CLI Flags
- `--fix`
- `-f`

## `from_ref`

Start reference for checking files.

- **Type**: `string`
- **Default**: `String("")`
- **Merge Policy**: `replace`

### CLI Flags
- `--from-ref`

## `glob`

Run on files that match these glob patterns.

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `replace`

### CLI Flags
- `--glob`
- `-g`

## `hide_warnings`

Warning tags to hide (union across sources).

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `union`

### PKL Sources
- `hide_warnings`

### Environment Variables
- `HK_HIDE_WARNINGS`

### Git Config Keys
- `hk.hideWarnings`

## `hide_when_done`

Hide output after successful completion.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_HIDE_WHEN_DONE`

## `hkrc`

Path to user configuration file.

- **Type**: `path`
- **Default**: `String(".hkrc.pkl")`
- **Merge Policy**: `replace`

### CLI Flags
- `--hkrc`

## `jobs`

Maximum number of parallel jobs to run. 0 means auto-detect.

- **Type**: `int`
- **Default**: `Integer(0)`
- **Merge Policy**: `replace`

### PKL Sources
- `defaults.jobs`

### Environment Variables
- `HK_JOBS`
- `HK_JOB`

### Git Config Keys
- `hk.jobs`

### CLI Flags
- `--jobs`
- `-j`

## `json`

Output in JSON format.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_JSON`

### Git Config Keys
- `hk.json`

### CLI Flags
- `--json`

## `libgit2`

Use libgit2 instead of git CLI commands.

- **Type**: `bool`
- **Default**: `Boolean(true)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_LIBGIT2`

## `log_file`

Path to log file.

- **Type**: `path`
- **Default**: `String("")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_LOG_FILE`

## `log_file_level`

Log level for file output.

- **Type**: `enum`
- **Default**: `String("info")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_LOG_FILE_LEVEL`

### Valid Values
- `off`
- `error`
- `warn`
- `info`
- `debug`
- `trace`

## `log_level`

Log level for console output.

- **Type**: `enum`
- **Default**: `String("info")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_LOG`
- `HK_LOG_LEVEL`

### Valid Values
- `off`
- `error`
- `warn`
- `info`
- `debug`
- `trace`

## `mise`

Enable mise integration.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_MISE`

## `no_progress`

Disable progress output.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### CLI Flags
- `--no-progress`

## `plan`

Print the plan instead of executing.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### CLI Flags
- `--plan`
- `-P`

## `profiles`

Profiles to enable/disable. Prefix with '!' to disable.

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `replace`

### PKL Sources
- `defaults.profiles`

### Environment Variables
- `HK_PROFILE`
- `HK_PROFILES`

### Git Config Keys
- `hk.profile`

### CLI Flags
- `--profile`
- `-p`

## `quiet`

Suppress output.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### CLI Flags
- `--quiet`
- `-q`

## `silent`

Suppress all output.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### CLI Flags
- `--silent`

## `skip_hooks`

Skip entire hooks (union across sources).

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `union`

### PKL Sources
- `defaults.skip_hooks`

### Environment Variables
- `HK_SKIP_HOOK`
- `HK_SKIP_HOOKS`

### Git Config Keys
- `hk.skipHooks`
- `hk.skipHook`

## `skip_steps`

Skip specific steps across all hooks (union across sources).

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `union`

### PKL Sources
- `defaults.skip_steps`

### Environment Variables
- `HK_SKIP_STEPS`
- `HK_SKIP_STEP`

### Git Config Keys
- `hk.skipSteps`
- `hk.skipStep`

### CLI Flags
- `--skip-step`

## `slow`

Enable slow mode. Shorthand for --profile=slow.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### CLI Flags
- `--slow`
- `-s`

## `stash`

Stash method to use for git hooks.

- **Type**: `enum`
- **Default**: `String("auto")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_STASH`

### Git Config Keys
- `hk.stash`

### CLI Flags
- `--stash`

### Valid Values
- `auto`
- `git`
- `patch-file`
- `none`

## `stash_untracked`

Include untracked files when stashing.

- **Type**: `bool`
- **Default**: `Boolean(true)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_STASH_UNTRACKED`

### Git Config Keys
- `hk.stashUntracked`

## `state_dir`

Directory for state files.

- **Type**: `path`
- **Default**: `String("")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_STATE_DIR`

## `step`

Run only specific step(s).

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `replace`

### CLI Flags
- `--step`
- `-S`

## `summary_text`

Allow output summaries to be printed in text mode.

- **Type**: `bool`
- **Default**: `Boolean(false)`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_SUMMARY_TEXT`

## `timing_json`

Path to write JSON timing report after hook finishes.

- **Type**: `path`
- **Default**: `String("")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_TIMING_JSON`

## `to_ref`

End reference for checking files.

- **Type**: `string`
- **Default**: `String("")`
- **Merge Policy**: `replace`

### CLI Flags
- `--to-ref`

## `trace`

Enable tracing spans and performance diagnostics.

- **Type**: `enum`
- **Default**: `String("off")`
- **Merge Policy**: `replace`

### Environment Variables
- `HK_TRACE`

### Git Config Keys
- `hk.trace`

### CLI Flags
- `--trace`

### Valid Values
- `off`
- `text`
- `json`
- `1`
- `true`

## `verbose`

Verbose output level.

- **Type**: `int`
- **Default**: `Integer(0)`
- **Merge Policy**: `replace`

### CLI Flags
- `--verbose`
- `-v`

## `warnings`

Warning tags to enable/show.

- **Type**: `list<string>`
- **Default**: `Array([])`
- **Merge Policy**: `replace`

### PKL Sources
- `warnings`

### Environment Variables
- `HK_WARNINGS`

### Git Config Keys
- `hk.warnings`

