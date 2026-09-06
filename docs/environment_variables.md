---
outline: [2, 2]
description: Configure hk with environment variables for files, profiles, execution, logging, Pkl evaluation, and stashing.
---

# Environment variables

Use environment variables for a single invocation or an environment-wide preference. Runtime settings generally override Git and Pkl settings; CLI flags take precedence. See [configuration precedence](/configuration#configuration-precedence).

Boolean settings accept `1`/`true` and `0`/`false`. List settings use commas. Set hk’s variables in the environment that launches hk; hook and step `env` blocks are for child commands.

Common overrides:

```sh
HK_PROFILE=slow hk check --all
HK_SKIP_STEPS=eslint hk check
HK_LOG=debug hk run pre-commit
```

## `HK` {#hk}

**Type:** boolean · **Default:** enabled in installed launchers

Set `HK=0` to bypass hk’s installed Git hook launcher for one command, for example `HK=0 git commit`. This is handled by the launcher, not by every direct hk invocation.

## `HK_CACHE` {#hk-cache}

**Type:** boolean · **Default:** true in release builds; false in debug builds

Enable or disable the evaluated-configuration cache. Use `HK_CACHE=0 hk validate` when diagnosing stale configuration, or `hk cache clear` to remove hk’s cache.

## `HK_CACHE_DIR` {#hk-cache-dir}

**Type:** path · **Default:** platform cache directory plus `hk`

Directory for cached configuration and other cache files. On Linux this is typically `~/.cache/hk`; on macOS it is typically `~/Library/Caches/hk`.

## `HK_CHECK` {#hk-check}

**Type:** boolean · **Default:** false

Request check commands instead of fix commands. This setting wins when both `HK_CHECK` and `HK_FIX` are enabled; explicit `--check` and `--fix` flags take precedence. Check commands should leave files unchanged, but hk does not enforce that convention.

## `HK_CHECK_FIRST` {#hk-check-first}

**Type:** boolean · **Default:** true

Allow read-only checks before fixes when steps overlap. A passing check can avoid a write lock; a failing check can narrow the files that need fixing. The step’s `check_first` setting also affects this optimization.

## `HK_CONFIG_DIR` {#hk-config-dir}

**Type:** path · **Default:** `$XDG_CONFIG_HOME/hk` or `~/.config/hk`

Directory containing the user `config.pkl`. See [user configuration](/configuration#hkrc) for discovery and legacy paths.

## `HK_DISPLAY_SKIP_REASONS` {#hk-display-skip-reasons}

**Type:** comma-separated strings · **Default:** `profile-not-enabled`

Select skip reasons to display. Use `hk check --why <step>` for a detailed explanation of a particular step’s selection.

## `HK_EXCLUDE` {#hk-exclude}

**Type:** comma-separated patterns · **Default:** empty

Exclude files or directories from processing. These patterns combine with exclusions from other sources. Example: `HK_EXCLUDE='node_modules,dist,**/*.min.js' hk check --all`.

## `HK_FAIL_FAST` {#hk-fail-fast}

**Type:** boolean · **Default:** true

Stop remaining work after a failure. Use `HK_FAIL_FAST=0 hk check --all` or `--no-fail-fast` to collect failures from remaining steps.

## `HK_FILE` {#hk-file}

**Type:** path · **Default:** automatic project config discovery

Select a specific Pkl configuration instead of searching for `hk.local.pkl` or `hk.pkl`. Example: `HK_FILE=./config/ci.pkl hk check --all`.

## `HK_FIX` {#hk-fix}

**Type:** boolean · **Default:** true

Permit fix mode when the hook requests it. Setting this to `false` disables configured fixes unless an explicit fix flag overrides it. A value of `true` does not make a normal `hk check` run fixes. `HK_CHECK=1` takes precedence when both settings are enabled.

## `HK_HIDE_WARNINGS` {#hk-hide-warnings}

**Type:** comma-separated tags · **Default:** empty

Suppress named warning categories, such as `missing-profiles`. Suppressed tags combine across configuration sources.

## `HK_HIDE_WHEN_DONE` {#hk-hide-when-done}

**Type:** boolean · **Default:** false

Hide progress output after a successful hook finishes. Failed runs keep their diagnostics.

## `HK_JOBS` {#hk-jobs}

**Type:** nonnegative integer · **Default:** 0 (detect CPU count)

Limit concurrent hk jobs. Example: `HK_JOBS=4 hk check --all`. Linters can also start their own workers, so increasing this value does not always improve speed.

## `HK_JSON` {#hk-json}

**Type:** boolean · **Default:** false

Request JSON output for commands that support it. For execution plans, use `hk check --plan --json`. For trace events, use `HK_TRACE=json`; this setting does not turn arbitrary linter output into structured results.

## `HK_LIBGIT2` {#hk-libgit2}

**Type:** boolean · **Default:** true

Use libgit2 for Git operations where supported. Set `HK_LIBGIT2=0` to use the Git CLI backend, for example when comparing performance with Git’s fsmonitor integration.

## `HK_LOG` {#hk-log}

**Type:** log level · **Default:** `info`

Console log level: `off`, `error`, `warn`, `info`, `debug`, or `trace`. `HK_LOG_LEVEL` is also accepted. Use `hk check -v` for debug output or `-vv` for trace logging.

## `HK_LOG_FILE` {#hk-log-file}

**Type:** path · **Default:** `$HK_STATE_DIR/hk.log`

Log file location. Example: `HK_LOG_FILE=/tmp/hk.log hk check`.

## `HK_LOG_FILE_LEVEL` {#hk-log-file-level}

**Type:** log level · **Default:** the environment’s log level

Choose a separate verbosity for file logs. Example: `HK_LOG_FILE_LEVEL=trace hk check`.

## `HK_MISE` {#hk-mise}

**Type:** boolean · **Default:** false

Make `hk install` use `mise x` in hook launchers and make `hk init` create a starter `mise.toml` when absent. Reinstall hooks to update an existing launcher. See [mise integration](/mise_integration). Steps also receive the mise environment for their working directory; explicit step environment values take precedence. See [per-directory environments](/mise_integration#per-directory-environments-monorepos).

## `HK_OUTPUT_FILE` {#hk-output-file}

Type: `path`
Default: `~/.local/state/hk/output.log`

The file where hk writes the complete output of a failed command. An empty value uses the default location.

## `HK_PKL_BACKEND` {#hk-pkl-backend}

**Type:** `pklr` or `pkl` · **Default:** `pklr`

Choose the built-in evaluator or the separately installed Pkl CLI. Example: `HK_PKL_BACKEND=pkl hk validate`. See [Pkl evaluators](/pkl_introduction#evaluators).

## `HK_PKL_CACHE_DIR` {#hk-pkl-cache-dir}

Type: `path`
Default: the platform cache directory with `pklr` appended (`~/.cache/pklr` on Linux, `~/Library/Caches/pklr` on macOS, and `%LOCALAPPDATA%\pklr` on Windows). Falls back to `~/.cache/pklr` when the platform cache directory is unavailable.

The directory used by the built-in pklr evaluator to persist downloaded Pkl packages. Packages in this cache can be reused after hk's resolved configuration cache is invalidated, including in offline mode.

This variable is read directly from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`.

## `HK_PKL_CA_CERTIFICATES` {#hk-pkl-ca-certificates}

**Type:** path · **Default:** unset

With the Pkl CLI backend, pass a CA certificate file through Pkl’s `--ca-certificates` flag. This must be set before configuration is evaluated.

## `HK_PKL_EMBEDDED` {#hk-pkl-embedded}

Type: `bool`
Default: `true`

hk embeds the Pkl package for its own version and seeds `HK_PKL_CACHE_DIR` with it before evaluating `hk.pkl`. A config pinning the running hk version therefore evaluates without a network request, even on a cold cache. A config pinning any other version is downloaded as usual.

Set to `0` to disable seeding. The package is then resolved from `HK_PKL_CACHE_DIR`, falling back to the network unless `HK_PKL_OFFLINE` is set, in which case a package missing from the cache fails.

This variable is read directly from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`.

## `HK_PKL_HTTP_REWRITE` {#hk-pkl-http-rewrite}

**Type:** string · **Default:** unset

With the Pkl CLI backend, pass a URL rewrite through Pkl’s `--http-rewrite` flag. The value has the form `https://source.example/=https://mirror.example/` and must be set before evaluation.

## `HK_PKL_OFFLINE` {#hk-pkl-offline}

Type: `bool`
Default: `false`

Disables network access in the built-in pklr evaluator. Package imports already present in `HK_PKL_CACHE_DIR`, along with the package embedded for the running version (see `HK_PKL_EMBEDDED`), remain available; a missing package fails immediately with its URL and cache location.

This variable is read directly from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`.

## `HK_PROFILE` {#hk-profile}

**Type:** comma-separated profile names · **Default:** empty

Enable profiles such as `slow` or `types`. Prefix a name with `!` to disable it. `HK_PROFILES` is also accepted. A step requires all of its positive profiles. Example: `HK_PROFILE=ci,slow hk check --all`.

## `HK_SKIP_HOOK` {#hk-skip-hook}

**Type:** comma-separated hook names · **Default:** empty

Skip entire hooks, for example `HK_SKIP_HOOK=pre-push git push`. `HK_SKIP_HOOKS` is also accepted. Skip lists combine with Git and Pkl configuration.

## `HK_SKIP_STEPS` {#hk-skip-steps}

**Type:** comma-separated step names · **Default:** empty

Skip named steps in any hook, for example `HK_SKIP_STEPS=eslint hk check`. `HK_SKIP_STEP` is also accepted. Skip lists combine across configuration sources.

## `HK_STAGE` {#hk-stage}

**Type:** boolean · **Default:** the hook’s staging setting

Override automatic staging of fixes. Set `HK_STAGE=0` to leave fixes for review. See [reviewing fixes](/hooks#review-fixes-before-committing).

## `HK_STASH` {#hk-stash}

**Type:** `git`, `patch-file`, or `none` · **Default:** the hook’s setting, otherwise `none`

Override how unstaged work is saved before a hook. `git` enables stashing; `patch-file` currently uses the same Git implementation; `none` leaves unstaged work in place. Boolean `true`/`1` and `false`/`0` are also accepted. `hk init` explicitly configures Git stashing for pre-commit. See [stashing](/hooks#stashing-and-partial-commits).

## `HK_STASH_BACKUP_COUNT` {#hk-stash-backup-count}

**Type:** nonnegative integer · **Default:** 20

Number of backup patches to retain per repository under `$HK_STATE_DIR/patches/`. Set to `0` to disable patch backups.

## `HK_STASH_UNTRACKED` {#hk-stash-untracked}

**Type:** boolean · **Default:** true

Include untracked files when stashing. Setting this to `false` also skips untracked-file discovery entirely: those files will not appear in status-based reports or normal `hk check --all` selection. This can reduce scan time for very large worktrees, such as dotfiles repositories rooted at the home directory.

## `HK_STATE_DIR` {#hk-state-dir}

**Type:** path · **Default:** platform state directory plus `hk`

Directory for logs and stash backup patches. It typically resolves to `~/.local/state/hk` on Linux; hk also uses that fallback on platforms without a state-directory convention.

## `HK_SUMMARY_TEXT` {#hk-summary-text}

**Type:** boolean · **Default:** false

In plain-text mode, hk prints summaries for failed steps by default. Set to `true` to include successful-step summaries too; their output normally streams during execution.

## `HK_TERMINAL_PROGRESS` {#hk-terminal-progress}

**Type:** boolean · **Default:** true

Send progress updates through OSC sequences to compatible terminals. Disable this if the terminal renders those updates incorrectly.

## `HK_TIMING_JSON` {#hk-timing-json}

**Type:** path · **Default:** unset

Write total and per-step wall time as JSON after a hook finishes. Example: `HK_TIMING_JSON=hk-timing.json hk check --all`. See [timing reports](/logging#a-run-is-slow).

## `HK_TRACE` {#hk-trace}

**Type:** `1`, `true`, or `json` · **Default:** off

Enable text tracing with `HK_TRACE=1`, or JSON trace events with `HK_TRACE=json`. Text goes to standard error; JSON events go to standard output. Other values, including `off`, disable tracing; `text` is not an alias for `1`. See [tracing](/logging#tracing).

## `HK_WALK_IGNORE` {#hk-walk-ignore}

**Type:** boolean · **Default:** true

Respect `.gitignore` and other ignore files during directory walks. This affects discovery; other file filters and step exclusions still apply.

## `HK_WARNINGS` {#hk-warnings}

**Type:** comma-separated warning tags · **Default:** empty

Enable opt-in warning categories, currently including `missing-profiles`. In Pkl, use `warnings = List("missing-profiles")`.

## `HK_REPORT_JSON` {#hk-report-json}

hk sets this variable for a hook’s `report` command. It contains the same timing data that `HK_TIMING_JSON` writes to a file. It is an output supplied to the report command, not a setting for users to configure.

```pkl
report = "node scripts/report-timings.js"
```

The script can read `process.env.HK_REPORT_JSON`. See [timing reports](/logging#a-run-is-slow) for the JSON shape.
