---
outline: [2, 2]
description: The standing orders ye give hk through environment variables, for its files, watches (profiles), how the crew runs, the ship's log, Pkl evaluation, and stowing the hold.
sourceHash: 6d7bc0c67e1c
---

# Standing orders (environment variables)

Environment variables be hk's standing orders: give one for a single call, or set it across yer whole environment as a standing preference. Runtime settings generally override Git and Pkl settings; CLI flags outrank them all. See [who outranks whom: configuration precedence](/configuration#configuration-precedence).

Aye-or-nay (boolean) orders take `1`/`true` and `0`/`false`. List orders are split with commas. Set hk's variables in the environment that launches hk; the `env` blocks on hooks and steps are for the child commands the crew runs, not for hk itself.

The orders ye'll give most often:

```sh
HK_PROFILE=slow hk check --all
HK_SKIP_STEPS=eslint hk check
HK_LOG=debug hk run pre-commit
```

## `HK` {#hk}

**Type:** boolean · **Unless ye say otherwise:** enabled in installed launchers

Set `HK=0` to slip past hk's installed Git hook launcher for one command, so the bosun's pipe stays quiet: for instance, `HK=0 git commit`. 'Tis the launcher that heeds this order, not every direct call to hk.

## `HK_CACHE` {#hk-cache}

**Type:** boolean · **Unless ye say otherwise:** true in release builds; false in debug builds

Turns the cache of evaluated configuration on or off: hk's copy of the charts, already read. When ye're hunting stale configuration, run `HK_CACHE=0 hk validate`, or clear out hk's locker with `hk cache clear`.

## `HK_CACHE_DIR` {#hk-cache-dir}

**Type:** path · **Unless ye say otherwise:** platform cache directory plus `hk`

hk's locker, where it keeps cached configuration and other cache files. On Linux that's typically `~/.cache/hk`; on macOS it's typically `~/Library/Caches/hk`.

## `HK_CHECK` {#hk-check}

**Type:** boolean · **Unless ye say otherwise:** false

Orders the crew to inspect the cargo (check commands) instead of mending the canvas (fix commands). When both `HK_CHECK` and `HK_FIX` are enabled, this one wins; explicit `--check` and `--fix` flags outrank them both. Check commands should leave the cargo untouched, but hk does not enforce that convention.

## `HK_CHECK_FIRST` {#hk-check-first}

**Type:** boolean · **Unless ye say otherwise:** true

Lets a hand inspect the cargo before mending it, and skip the mending when the inspection passes. Steps do this when they set `check_first = true` (and another step writes the same files), and always when their `check` and `fix` are the same command, as in steps migrated from pre-commit. Set this to `false` and it's off for both; such a migrated fixer then fails whenever it fixes something, because it exits 1 after fixing.

Some hands check first whatever this order says, because their mending or staging depends on it: a step with `check_diff` in fix mode applies the diff instead of running `fix`, and in a hook that stages its fixes, a step with `check_diff` or `check_list_files` lists the files it would change, so only those are fixed and staged. With the default `stage`, that happens only when one of the step's files has unstaged changes that hk didn't stow in the hold.

## `HK_CONFIG_DIR` {#hk-config-dir}

**Type:** path · **Unless ye say otherwise:** `$XDG_CONFIG_HOME/hk` or `~/.config/hk`

The directory that holds yer user `config.pkl`, the charts ye carry from ship to ship. See [user configuration](/configuration#hkrc) for how hk finds it, and for the legacy paths.

## `HK_DISPLAY_SKIP_REASONS` {#hk-display-skip-reasons}

**Type:** comma-separated strings · **Unless ye say otherwise:** `profile-not-enabled`

Pick which reasons for skipping a hand get called out. For the full tale of how one particular step was picked or passed over, use `hk check --why <step>`.

## `HK_EXCLUDE` {#hk-exclude}

**Type:** comma-separated patterns · **Unless ye say otherwise:** empty

Strike files or directories off the manifest, so the crew doesn't process them. These patterns combine with exclusions from other sources. For example: `HK_EXCLUDE='node_modules,dist,**/*.min.js' hk check --all`.

## `HK_FAIL_FAST` {#hk-fail-fast}

**Type:** boolean · **Unless ye say otherwise:** true

At the first squall, stop the rest o' the work. To collect the failures from the remaining steps instead, use `HK_FAIL_FAST=0 hk check --all` or `--no-fail-fast`.

## `HK_FILE` {#hk-file}

**Type:** path · **Unless ye say otherwise:** automatic project config discovery

Hand hk one particular set of charts, a specific Pkl configuration, instead of letting it search for `hk.local.pkl` or `hk.pkl`. For example: `HK_FILE=./config/ci.pkl hk check --all`.

## `HK_FIX` {#hk-fix}

**Type:** boolean · **Unless ye say otherwise:** true

Lets the sailmakers mend the canvas when the hook calls for fix mode. Set it to `false` and the configured fixes stand down, unless an explicit fix flag overrides it. A value of `true` does not make a normal `hk check` run fixes: inspecting stays inspecting. `HK_CHECK=1` wins when both settings are enabled.

## `HK_HIDE_WARNINGS` {#hk-hide-warnings}

**Type:** comma-separated tags · **Unless ye say otherwise:** empty

Hush named categories of warning, such as `missing-profiles` or `local-config-replaces-shared`. The tags ye hush combine across configuration sources.

## `HK_HIDE_WHEN_DONE` {#hk-hide-when-done}

**Type:** boolean · **Unless ye say otherwise:** false

Clear the progress output off the deck once a hook finishes successfully. A failed run keeps its diagnostics on show.

## `HK_JOBS` {#hk-jobs}

**Type:** nonnegative integer · **Unless ye say otherwise:** 0 (detect CPU count)

All hands haul at once, and this limits how many hk jobs haul together. For example: `HK_JOBS=4 hk check --all`. Linters can also start their own workers, so raising this value does not always make the run faster.

## `HK_JSON` {#hk-json}

**Type:** boolean · **Unless ye say otherwise:** false

Asks for JSON output from the commands that support it. For the passage plan, use `hk check --plan --json`. For trace events, use `HK_TRACE=json`. This order does not turn whatever a linter prints into structured results.

## `HK_LIBGIT2` {#hk-libgit2}

**Type:** boolean · **Unless ye say otherwise:** true

Work Git through libgit2 wherever it's supported. Set `HK_LIBGIT2=0` to use the Git CLI backend instead, for instance when ye're comparing performance with Git's fsmonitor integration.

## `HK_LOG` {#hk-log}

**Type:** log level · **Unless ye say otherwise:** `info`

How much hk calls out on the console: `off`, `error`, `warn`, `info`, `debug`, or `trace`. It answers to `HK_LOG_LEVEL` too. Use `hk check -v` for debug output, or `-vv` for trace logging.

## `HK_LOG_FILE` {#hk-log-file}

**Type:** path · **Unless ye say otherwise:** `$HK_STATE_DIR/hk.log`

Where the ship's log is written. For example: `HK_LOG_FILE=/tmp/hk.log hk check`.

## `HK_LOG_FILE_LEVEL` {#hk-log-file-level}

**Type:** log level · **Unless ye say otherwise:** the environment's log level

Give the ship's log file its own verbosity, apart from the console. For example: `HK_LOG_FILE_LEVEL=trace hk check`.

## `HK_MISE` {#hk-mise}

**Type:** boolean · **Unless ye say otherwise:** false

Calls in the quartermaster: `hk install` uses `mise x` in the hook launchers, and `hk init` creates a starter `mise.toml` when there isn't one. Reinstall the hooks to update a launcher that's already rigged. See [mise integration](/mise_integration). Steps also receive the mise environment for their working directory; explicit step environment values take precedence. See [per-directory environments](/mise_integration#per-directory-environments-monorepos).

## `HK_OUTPUT_FILE` {#hk-output-file}

Type: `path`
Unless ye say otherwise: `~/.local/state/hk/output.log`

The file where hk writes the complete output of a failed command, every word o' the squall. An empty value uses the default location.

## `HK_PKL_BACKEND` {#hk-pkl-backend}

**Type:** `pklr` · **Unless ye say otherwise:** unset

hk v2 always reads its charts with its built-in pklr evaluator. The value `pklr` is accepted for compatibility and changes nothing; `pkl` and any other value fail with a link to the [v2 migration guide](/migration-v2).

## `HK_PKL_CACHE_DIR` {#hk-pkl-cache-dir}

Type: `path`
Unless ye say otherwise: the platform cache directory with `pklr` appended (`~/.cache/pklr` on Linux, `~/Library/Caches/pklr` on macOS, and `%LOCALAPPDATA%\pklr` on Windows). When the platform cache directory is unavailable, it falls back to `~/.cache/pklr`.

Where the built-in pklr evaluator keeps the Pkl packages it has downloaded, laid up in the locker for later runs. Packages in this cache can be used again after hk's resolved configuration cache is invalidated, in offline mode too.

hk reads this one straight from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`: the charts can't set an order that's read before they're opened.

## `HK_PKL_CA_CERTIFICATES` {#hk-pkl-ca-certificates}

**Type:** path · **Unless ye say otherwise:** unset

A path to a PEM bundle of CA certificates that the built-in pklr evaluator trusts, in addition to the system roots, the seals it will honour when it downloads Pkl packages. This is useful behind an SSL-intercepting proxy. This must be set before the configuration is evaluated.

## `HK_PKL_EMBEDDED` {#hk-pkl-embedded}

Type: `bool`
Unless ye say otherwise: `true`

hk carries the Pkl package for its own version aboard, and seeds `HK_PKL_CACHE_DIR` with it before evaluating `hk.pkl`. So a config pinning the running hk version evaluates without a network request, even on a cold cache. A config pinning any other version is downloaded as usual.

Set it to `0` to stop the seeding. The package is then resolved from `HK_PKL_CACHE_DIR`, falling back to the network, a boat sent ashore, unless `HK_PKL_OFFLINE` is set; in that case, a package missing from the cache fails.

hk reads this one straight from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`.

## `HK_PKL_HTTP_REWRITE` {#hk-pkl-http-rewrite}

**Type:** string · **Unless ye say otherwise:** unset

URL rewrite rules for the built-in pklr evaluator, so the boats it sends ashore land at another address. A rule has the form `https://source.example/=https://mirror.example/`: a URL that starts with the left side is fetched from the right side instead. Separate several rules with commas; when more than one matches, the longest source prefix wins. A rule with no `=` is ignored with a warning. The variable must be set before evaluation.

Package downloads are rewritten as the `https://host/path/name@version.zip` URL, so one rule can redirect a whole package host to a mirror. See [share configuration across repositories](/configuration#share-rewrite), including how to supply credentials.

## `HK_PKL_OFFLINE` {#hk-pkl-offline}

Type: `bool`
Unless ye say otherwise: `false`

Keeps the built-in pklr evaluator off the network: no boats go ashore. Package imports already in `HK_PKL_CACHE_DIR`, along with the package embedded for the running version (see `HK_PKL_EMBEDDED`), stay available; a missing package fails at once, with its URL and cache location.

hk reads this one straight from the environment before `hk.pkl` is evaluated, so it cannot be configured in `hk.pkl`.

## `HTTP_PROXY`, `HTTPS_PROXY`, `NO_PROXY` {#proxy-variables}

**Type:** URL, URL, host list · **Unless ye say otherwise:** unset

The built-in pklr evaluator sends its boats ashore, package and module downloads, through a proxy. It uses the first of `http_proxy`, `HTTP_PROXY`, `https_proxy`, and `HTTPS_PROXY` that is set and nonempty, for every download whatever its scheme. `no_proxy` or `NO_PROXY` lists hosts that bypass it. A proxy that intercepts TLS also needs [`HK_PKL_CA_CERTIFICATES`](#hk-pkl-ca-certificates).

## `HK_PROFILE` {#hk-profile}

**Type:** comma-separated profile names · **Unless ye say otherwise:** empty

Call up watches (profiles) such as `slow` or `types`. Put `!` in front of a name to stand that watch down. It answers to `HK_PROFILES` too. A step requires all of its positive profiles to be called up. For example: `HK_PROFILE=ci,slow hk check --all`.

## `HK_SKIP_HOOK` {#hk-skip-hook}

**Type:** comma-separated hook names · **Unless ye say otherwise:** empty

Skip entire hooks, so the bosun's pipe stays silent for them, as in `HK_SKIP_HOOK=pre-push git push`. It answers to `HK_SKIP_HOOKS` too. Skip lists combine with Git and Pkl configuration.

## `HK_SKIP_STEPS` {#hk-skip-steps}

**Type:** comma-separated step names · **Unless ye say otherwise:** empty

Skip named steps in any hook, so those hands sit this one out, as in `HK_SKIP_STEPS=eslint hk check`. It answers to `HK_SKIP_STEP` too. Skip lists combine across configuration sources.

## `HK_STAGE` {#hk-stage}

**Type:** boolean · **Unless ye say otherwise:** the hook's staging setting

Overrides the automatic staging of fixes, loading the mended canvas aboard. Set `HK_STAGE=0` to leave the fixes for ye to review. See [reviewing fixes](/hooks#review-fixes-before-committing).

## `HK_STASH` {#hk-stash}

**Type:** `git`, `patch-file`, or `none` · **Unless ye say otherwise:** the hook's setting, otherwise `none`

Overrides how unstaged work is stowed before a hook runs. `git` turns stashing on; `patch-file` currently uses the same Git implementation; `none` leaves unstaged work where it lies. Boolean `true`/`1` and `false`/`0` are accepted too. A hook defaults to `none` unless it sets `stash`; the `pre-commit` hook hk builds from top-level `steps` (the setup `hk init` creates) uses `git`. See [stowing the hold](/hooks#stashing-and-partial-commits).

## `HK_STASH_BACKUP_COUNT` {#hk-stash-backup-count}

**Type:** nonnegative integer · **Unless ye say otherwise:** 20

How many backup patches to keep for each ship (repository), under `$HK_STATE_DIR/patches/`. Set to `0` to turn patch backups off.

## `HK_STASH_UNTRACKED` {#hk-stash-untracked}

**Type:** boolean · **Unless ye say otherwise:** true

Stow untracked files in the hold too when stashing. Set this to `false` and hk also skips looking for untracked files entirely: they will not appear in status-based reports or in the normal `hk check --all` selection. That can cut scan time for very large worktrees, such as dotfiles repositories rooted at the home directory.

## `HK_STATE_DIR` {#hk-state-dir}

**Type:** path · **Unless ye say otherwise:** platform state directory plus `hk`

Where hk keeps the ship's logs and the stash backup patches. It typically resolves to `~/.local/state/hk` on Linux, and hk falls back to that same path on platforms with no state-directory convention.

## `HK_SUMMARY_TEXT` {#hk-summary-text}

**Type:** boolean · **Unless ye say otherwise:** false

In plain-text mode, hk prints summaries for failed steps by default. Set this to `true` to get summaries for the steps that succeeded too; their output normally streams past while the crew works.

## `HK_TERMINAL_PROGRESS` {#hk-terminal-progress}

**Type:** boolean · **Unless ye say otherwise:** true

Signal progress to compatible terminals through OSC sequences, like flags run up the mast. Turn it off if yer terminal renders those signals wrong.

## `HK_TIMING_JSON` {#hk-timing-json}

**Type:** path · **Unless ye say otherwise:** unset

After a hook finishes, write the total and per-step wall time as JSON: how long the whole run and each hand took. For example: `HK_TIMING_JSON=hk-timing.json hk check --all`. See [timing reports](/logging#a-run-is-slow).

## `HK_TRACE` {#hk-trace}

**Type:** `1`, `true`, or `json` · **Unless ye say otherwise:** off

Turn on text tracing with `HK_TRACE=1`, or JSON trace events with `HK_TRACE=json`. Text goes to standard error; JSON events go to standard output. Any other value, `off` included, turns tracing off, and mind that `text` is not an alias for `1`. See [tracing](/logging#tracing).

## `HK_WALK_IGNORE` {#hk-walk-ignore}

**Type:** boolean · **Unless ye say otherwise:** true

Heed `.gitignore` and the other ignore files while walking the directories for cargo. This affects what gets discovered; other file filters and step exclusions still apply.

## `HK_WARNINGS` {#hk-warnings}

**Type:** comma-separated warning tags · **Unless ye say otherwise:** empty

Turn on opt-in categories of warning, currently including `missing-profiles`. In the charts (Pkl), use `warnings = List("missing-profiles")`.

## `HK_REPORT_JSON` {#hk-report-json}

This order runs the other way: hk sets this variable for a hook's `report` command. It holds the same timing data that `HK_TIMING_JSON` writes to a file. It is an output handed to the report command, not a setting for ye to configure.

```pkl
report = "node scripts/report-timings.js"
```

The script can read `process.env.HK_REPORT_JSON`. See [timing reports](/logging#a-run-is-slow) for the shape of the JSON.
