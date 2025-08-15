---
outline: "deep"
---

# Environment Variables

Environment variables can be used to configure hk.

## `HK_CACHE_DIR`

Type: `path`
Default: `~/.cache/hk`

The cache directory to use.

## `HK_CHECK_FIRST`

Type: `bool`
Default: `true`

If true, hk will run check commands first then run fix commands if check fails iff there are multiple linters with the same file in a matching glob pattern.

The reason for this is to make hk able to parallelize as much as possible. We can have as many check commands running in parallel against
the same file as we want without them interfering with each other—however we can't have 2 fix commands potentially writing to the same file. So we optimistically run the check commands in parallel, then if any fail we run their fix commands potentially in series.

If this is disabled hk will have simpler logic that just uses fix commands in series in this situation.

## `HK_PROFILE`

Type: `string[]` (comma-separated list)

The profile(s) to use.

## `HK_FILE`

Type: `string`
Default: `hk.pkl` | `hk.toml` | `hk.yaml` | `hk.yml` | `hk.json`

The file to use for the configuration.

## `HK_FIX`

Type: `bool`
Default: `true`

If set to `false`, hk will not run fix steps.

## `HK_JOBS`

Type: `usize`
Default: `(number of cores)`

The number of jobs to run in parallel.

## `HK_LOG`

Type: `trace` | `debug` | `info` | `warn` | `error`
Default: `info`

The log level to use.

## `HK_LOG_FILE`

Type: `path`
Default: `~/.local/state/hk/hk.log`

The log file to use.

## `HK_LOG_FILE_LEVEL`

Type: `trace` | `debug` | `info` | `warn` | `error`
Default: `HK_LOG`

The log level to use for the log file.

## `HK_MISE`

Type: `bool`
Default: `false`

If set to `true`:

- When installing hooks with `hk install`, hk will use `mise x` to execute hooks which won't require activating mise to use mise tools
- When generating files with `hk init`, hk will create a `mise.toml` file with hk configured

## `HK_SKIP_STEPS`

Type: `string[]` (comma-separated list)

A comma-separated list of step names to skip when running pre-commit and pre-push hooks.
For example: `HK_SKIP_STEPS=lint,test` would skip any steps named "lint" or "test".

## `HK_SKIP_HOOK`

Type: `string[]` (comma-separated list)
Default: `(empty)`

A comma-separated list of hook names to skip entirely. This allows you to disable specific git hooks from running.
For example: `HK_SKIP_HOOK=pre-commit,pre-push` would skip running those hooks completely.

This is useful when you want to temporarily disable certain hooks while still keeping them configured in your `hk.pkl` file.
Unlike `HK_SKIP_STEPS` which skips individual steps, this skips the entire hook and all its steps.

## `HK_STASH`

Type: `git` | `patch-file` | `none`
Default: `git`

- `git`: Use `git stash` to stash unstaged changes before running hooks.
- `patch-file`: Use an hk generated patch file to stash unstaged changes before running hooks (typically faster and avoids `index is locked` errors).
- `none`: Do not stash unstaged changes before running hooks. Much faster but will stage unstaged changes if they are in the same file as staged changes with fix modifications.

## `HK_STASH_UNTRACKED`

Type: `bool`
Default: `true`

If set to `true`, hk will stash untracked files when stashing before running hooks.

## `HK_FAIL_FAST`

Type: `bool`
Default: `true`

If `true`, hk will abort running steps after the first one fails.

## `HK_STATE_DIR`

Type: `path`
Default: `~/.local/state/hk`

The state directory to use.

## `HK_HIDE_WHEN_DONE`

Type: `bool`
Default: `false`

If set to `true`, hk will hide the progress output when the hook finishes if there are no errors.

## `HK_LIBGIT2`

Type: `bool`
Default: `true`

If set to `false`, hk will not use libgit2 to interact with git and instead use shelling out to git commands. This may provide better performance
in some cases such as when using `fsmonitor` to watch for changes.

## `HK_TIMING_JSON`

Type: `path`

If set to a file path, hk will write a JSON timing report at the end of a run. The report includes total wall time and per-step wall time, with overlapping intervals merged so time isn’t double-counted across parallel step parts.

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
