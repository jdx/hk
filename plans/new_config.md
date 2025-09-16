## Goals

- **Unify configuration** across CLI flags, environment variables, gitconfig, user rc (`.hkrc.pkl`), and project config (`hk.pkl` and friends).
- **Clear precedence** rules for runtime behavior vs. project structure.
- **Union semantics for excludes**: paths and globs should merge from all sources (never override).
- **Minimal architecture change**: extend current `src/settings.rs` and existing init flow; avoid introducing a new resolver subsystem or feature flags.
- **Introspection**: optional commands to see effective values and their sources.

## Configuration sources in scope

- **CLI flags**: parsed in `src/cli/mod.rs` and subcommands.
- **Environment variables**: `HK_*` in `src/env.rs`.
- **Git config**: new; read from git’s config stack (`.git/config`, `~/.gitconfig`, system) under `[hk]`.
- **User rc**: `.hkrc.pkl` loaded via `Config::apply_user_config`.
- **Project config**: `hk.pkl` (and `hk.toml|yaml|json`) loaded first.

## Precedence model

- **Runtime/settings-type keys** (e.g., jobs, profiles, fail_fast, tracing/json output, logging, stash method, warnings):
  1. CLI flags
  2. Environment (`HK_*`)
  3. Git config (local repo)
  4. User rc (`.hkrc.pkl`)
  5. Git config (global/system)
  6. Project config defaults
  7. Built-in defaults

- **Project structure** (hooks/steps topology, commands): comes from project config; `.hkrc.pkl` may overlay limited aspects (already supported: env, step filters), but CLI/env/gitconfig must not mutate the graph.

- **Special case: excludes**: union across all sources. No overriding. See “Exclude merging” below.

## Canonical key mapping

- **jobs**: CLI `--jobs`; env `HK_JOBS`; git `[hk] jobs`; rc `defaults.jobs`; project none ⇒ `Settings.jobs`.
- **profiles**: CLI `--profile`/`--slow`; env `HK_PROFILE`; git `[hk] profile` (repeatable or comma-separated); rc `defaults.profiles` ⇒ `Settings.enabled_profiles`/`disabled_profiles`.
- **fail_fast**: CLI `--fail-fast/--no-fail-fast` (add); env `HK_FAIL_FAST`; git `[hk] failFast`; rc `defaults.fail_fast`; project `fail_fast` ⇒ `Settings.fail_fast`.
- **fix/check mode defaults**: CLI `--fix/--check`; env `HK_FIX`; git `[hk] fix` (bool); rc `defaults.fix`/`defaults.check`; project hook-level `fix` ⇒ runtime behavior.
- **stash method**: CLI `--stash=<git|patch-file|none>` (add); env `HK_STASH`; git `[hk] stash`; rc (add `defaults.stash`); project hook-level `stash`.
- **stash untracked**: env `HK_STASH_UNTRACKED`; git `[hk] stashUntracked`; rc (optional); project hook-level override remains.
- **check_first**: env `HK_CHECK_FIRST`; git `[hk] checkFirst`; rc optional; project step-level `check_first` remains.
- **default_branch**: git `[hk] defaultBranch`; project `default_branch`.
- **json/trace**: CLI `--json`/`--trace`; env `HK_JSON`/`HK_TRACE`; git `[hk] json`/`trace`.
- **logging**: env `HK_LOG`, `HK_LOG_FILE`, `HK_LOG_FILE_LEVEL`; git `[hk] log`, `logFile`, `logFileLevel`.
- **summaries**: env `HK_SUMMARY_TEXT`; git `[hk] summaryText`.
- **warnings/hide_warnings**: env `HK_HIDE_WARNINGS`; rc `warnings`/`hide_warnings`; git `[hk] warnings`/`hideWarnings`.
- **hk file path**: env `HK_FILE`; git `[hk] file`.
- **hkrc path**: CLI `--hkrc` only (remains explicit).
- **exclude paths**: CLI `--exclude`; env `HK_EXCLUDE`; git `[hk] exclude`; rc `defaults.exclude` (new); project `exclude` (optional global) ⇒ union.
- **exclude globs**: CLI `--exclude-glob`; env `HK_EXCLUDE_GLOB`; git `[hk] excludeGlob`; rc `defaults.exclude_glob` (new); project `exclude_glob` (optional global) ⇒ union.

## Design approach (no new resolver)

- Keep `Settings` as the central runtime snapshot. Extend it to hold excludes and accept inputs from multiple sources.
- Continue loading order: project config → set `.hkrc.pkl` path from CLI → user rc overlay → runtime flags/env applied. Add gitconfig reads early in CLI `run()`.
- Use existing `Settings::set_*` where appropriate. For excludes, add merge APIs that union values from each source.
- Constrain `Config::init` to avoid overriding finalized hk settings with `HK_*` coming from `Config.env`; only set `HK_*` if not already decided by higher-precedence inputs. Non-`HK_*` env always set for child processes.

## Exclude merging

- Add to `src/settings.rs`:
  - Snapshot fields: `exclude_paths: IndexSet<PathBuf>`, `exclude_globs: IndexSet<String>`.
  - Statics: `EXCLUDE_PATHS`, `EXCLUDE_GLOBS` as `Mutex<Option<IndexSet<...>>>`.
  - APIs:
    - `add_exclude_paths<I: IntoIterator<Item = PathBuf>>(I)` → union into `EXCLUDE_PATHS`.
    - `add_exclude_globs<I: IntoIterator<Item = String>>(I)` → union into `EXCLUDE_GLOBS`.
    - Accessors in `Default` impl copy these into the snapshot; merge with env defaults if not set.
- Read env defaults in `Default`: `HK_EXCLUDE` (comma-separated paths) and `HK_EXCLUDE_GLOB`.
- In `Hook::file_list`, before filtering, compute the union of:
  - `Settings::get().exclude_paths` + `opts.exclude`
  - `Settings::get().exclude_globs` + `opts.exclude_glob`
  Then apply existing filtering logic once with the combined sets.

## Git config support

- Add a small helper (e.g., `git_cfg.rs`) to read `[hk]` values via `git2::Config::open_default()`:
  - Strings/ints/bools: `jobs`, `failFast`, `fix`, `stash`, `stashUntracked`, `checkFirst`, `defaultBranch`, `json`, `trace`, `log`, `logFile`, `logFileLevel`, `summaryText`, `warnings`, `hideWarnings`, `file`.
  - Lists: `profile`, `warnings`, `hideWarnings`, `exclude`, `excludeGlob` (support repeatable and comma-separated).
- Wire reads in `cli::run()` right after `Settings::set_user_config_path(...)` and before command dispatch. For excludes, call `Settings::add_exclude_*` (union). For other keys, set via existing `Settings` setters if not already determined by CLI/env (respect precedence).

## `.hkrc.pkl` and `hk.pkl` schema tweaks

- In `pkl/UserConfig.pkl` and docs, add `defaults.exclude` and `defaults.exclude_glob` supporting `String | List<String>`.
- In `Config::apply_user_config`, if provided, call `Settings::add_exclude_paths` / `add_exclude_globs`.
- Optionally (if desired), add root-level `exclude` / `exclude_glob` to `Config` for project-wide global excludes, merged via `Settings::add_*` during `Config::init`.

## CLI additions (optional but recommended)

- Add `--fail-fast` / `--no-fail-fast` and `--stash=<mode>` for parity with other sources.
- Add `hk config` subcommands (introspection):
  - `hk config dump` → print effective runtime settings (JSON/TOML).
  - `hk config get <key>` → value + source.
  - `hk config sources` → per-key origin summary.

## Implementation steps

1. Settings and env
   - Extend `src/settings.rs` with exclude fields, statics, union APIs, and wire env defaults in `Default` using `HK_EXCLUDE`, `HK_EXCLUDE_GLOB`.
   - Extend `src/env.rs` with parsers for `HK_EXCLUDE` (paths) and `HK_EXCLUDE_GLOB` (strings).
2. Hook integration
   - Update `Hook::file_list` to union settings excludes with CLI `--exclude/--exclude-glob` and filter once.
3. Git config (phase 1)
   - Implement helper to read `[hk]` and set `Settings` values. Focus on `profile`, `jobs`, `failFast`, and excludes first.
4. `.hkrc.pkl` (phase 1)
   - Add `defaults.exclude` and `defaults.exclude_glob`. Update `Config::apply_user_config` to union them.
5. Docs
   - Update `docs/configuration.md`, `docs/environment_variables.md` with keys, precedence, and exclude union semantics.
   - Add a small “Git config” section with examples.
6. Optional project-level excludes (phase 2)
   - Add `exclude` / `exclude_glob` to `Config` root and merge in `Config::init`.
7. Introspection (phase 2)
   - Add `hk config` commands to print effective values and sources.

## Testing

- Unit tests
  - `settings` union APIs add paths/globs without duplication and are order-insensitive.
  - Env parsing for `HK_EXCLUDE` and `HK_EXCLUDE_GLOB`.
- Integration tests (bats/rust)
  - Verify that excludes from env + `.hkrc.pkl` + gitconfig + CLI all union and filter the file set.
  - Precedence sanity for non-union keys: CLI > env > git (local) > hkrc > git (global) > project > default.
  - Ensure `Config.env` with `HK_*` does not override already-determined settings.

## Backwards compatibility

- Existing behavior preserved for all keys except new union of excludes (only additive for users who set them in multiple places).
- No new resolver or feature flag; small, incremental changes to `settings`, `env`, and `hook`.

## Git config examples

```ini
[hk]
  jobs = 8
  profile = slow
  profile = !experimental
  failFast = true
  stash = patch-file
  exclude = node_modules
  exclude = target
  excludeGlob = **/*.min.js
```

## Open questions (to decide during implementation)

- Should project-level global `exclude`/`exclude_glob` be supported, or keep excludes strictly runtime/user-level?
- For list keys like `warnings`/`hideWarnings`, do we want union or last-wins? (Current plan: keep last-wins for non-exclude lists to maintain existing semantics.)
