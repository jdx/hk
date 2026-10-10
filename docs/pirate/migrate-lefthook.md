---
description: Translate a lefthook.yml into hk.pkl by hand, with a map of options, placeholders, and commands, and the differences in behavior to check before ye sail.
sourceHash: d087f7d49f9b
---

# Changing ships from lefthook

hk has no converter for lefthook. (`hk migrate pre-commit` handles only the pre-commit framework.) Ye translate `lefthook.yml` into `hk.pkl` by hand, matey. The two hook managers share the same shape: hooks that hold named jobs that select the cargo and run commands. So most charts map one to one.

## Differences to check first {#differences-to-check-first}

- **Parallelism.** lefthook runs a hook's jobs in sequence unless ye set `parallel: true`. hk has all hands haul at once by default, up to the job limit. It lashes steps that select the same files with read and write locks, and ye declare an order with `depends`. A lefthook chart that relies on its sequence needs `depends` in hk. See [order steps deliberately](/hooks#order-steps-deliberately).
- **Check and fix.** A lefthook `run` is one command. hk separates `check`, which inspects the cargo, from `fix`, which mends the canvas. A formatter that rewrites files in `pre-commit` becomes a `fix` command in a hook with `fix = true`.
- **Staging fixes.** lefthook loads changed files aboard for jobs with `stage_fixed: true`. hk's `pre-commit` hook stages the files a step fixed by default, and a step's `stage` globs narrow or widen that. Other hooks leave fixes unstaged unless `stage = true`.
- **Partial voyages.** `stash = "git"` on a hook makes hk stow unstaged changes in the hold while it runs and bring them up afterward, so the lookouts see the staged content. See [stashing and partial commits](/hooks#stashing-and-partial-commits).
- **Failures.** hk stops after the first failing step by default (`fail_fast`). `hk run pre-commit --no-fail-fast` reports every squall.
- **Globs.** Both tools match a pattern without a slash against files at any depth, so `*.js` matches `a.js` and `src/b.js` in each. They differ for `**/*.js`: lefthook matches only files below a directory, and hk also matches `a.js` at the root of the ship.

## Changing ships {#move-over}

1. Write `hk.pkl` from yer `lefthook.yml`, using the map below. `hk init` can generate a starting chart for the tools it detects. See [getting started](/getting_started).
2. Run `hk validate`, then preview what a hook selects with `hk run pre-commit --plan`.
3. Take down lefthook's Git hooks with `lefthook uninstall`. That deletes the hooks it installed in `.git/hooks`. It leaves `lefthook.yml` unless ye pass `--remove-configs`, so ye can keep it for reference.
4. Run `hk install`. On Git 2.54 and newer hk registers its hooks in Git config, which runs alongside any other hook manager, so take down lefthook's hooks first or both sound.
5. Delete `lefthook.yml`, `lefthook-local.yml`, and the lefthook dependency or tool pin.

## Hooks, jobs, and options {#hooks-jobs-and-options}

| lefthook                                  | hk                                                                                                                  |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `pre-commit`, `commit-msg`, `pre-push`    | A hook of the same name under `hooks`. See [other Git events](/hooks#other-git-events) for the others hk handles.   |
| `commands.<name>` or a `jobs` entry       | `steps { ["<name>"] { ... } }`                                                                                      |
| `run`                                     | `check` for a command that inspects, `fix` for one that mends files                                                 |
| `glob`                                    | `glob`, a string or `List`. A `Regex` also works.                                                                   |
| `exclude`                                 | `exclude`                                                                                                           |
| `root`                                    | `dir`                                                                                                               |
| `env`                                     | `env` on the step, or on the hook for every step in it                                                              |
| `parallel: true`                          | The default                                                                                                         |
| `piped: true`, `priority`                 | `depends` between steps                                                                                             |
| `stage_fixed: true`                       | The `pre-commit` default, or `stage` on the hook or step                                                            |
| `skip`, `only`                            | `step_condition` or `condition`, or `profiles`. See [conditions](#conditions).                                      |
| `tags`                                    | `profiles`. A step with `profiles` sits out unless ye call up one of them with `--profile` or `HK_PROFILE`.         |
| `interactive: true`                       | `interactive = true`                                                                                                |
| `scripts`                                 | A step whose `check` or `fix` runs the script, for example `check = "./scripts/lint.sh {{files}}"`                  |
| `lefthook-local.yml`                      | `hk.local.pkl`, which amends `hk.pkl`. See [`hk.local.pkl`](/configuration#hk-local-pkl).                           |
| `extends`, `remotes`                      | Pkl `amends` and `import`. See [Pkl essentials](/pkl_introduction).                                                 |
| `fail_text`                               | No equivalent                                                                                                       |

## Placeholders {#placeholders}

hk renders commands as [Tera](https://keats.github.io/tera/) templates, so a placeholder is written with double braces.

| lefthook                       | hk                                                                                                   |
| ------------------------------ | ---------------------------------------------------------------------------------------------------- |
| `{staged_files}` in pre-commit | `{{files}}`, the files the hook selected that match the step. In `pre-commit` they are staged.       |
| `{push_files}` in pre-push     | `{{files}}`. In `pre-push` hk selects the files changed by the push.                                 |
| `{1}` in commit-msg            | `{{commit_msg_file}}`                                                                                |
| `{1} {2}` in pre-push          | `{{hook_args}}`, the remote name and URL                                                             |
| `{all_files}`                  | No placeholder. Use a command that finds its own files, or run `hk run pre-commit --all`.            |

`{{files}}` is quoted for the shell and relative to the step's `dir`. See [define a step](/configuration#define-a-step) and [commit-message hooks](/hooks#commit-message-hooks).

## Conditions {#conditions}

lefthook's `skip: [merge]` has no built-in in hk. Write the condition yerself. `step_condition` is an expression, and `exec_ok(...)` is true when a shell command exits with status 0:

```pkl
["lint"] {
  // Skip while a merge is in progress
  step_condition = "!exec_ok('git rev-parse -q --verify MERGE_HEAD')"
  check = "make lint"
}
["release-notes"] {
  // Run only on main
  step_condition = "exec_ok('test \"$(git branch --show-current)\" = main')"
  check = "make release-notes"
}
```

Use `exec_ok` to test whether a command succeeds. `exec(...)` returns a command's output instead, and a command that exits non-zero makes `exec` fail the hook. See [conditions and Git status](/configuration#conditions-and-git-status).

## Commands and skipping {#commands-and-skipping}

| lefthook                                    | hk                                                                                                                                      |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `lefthook install`                          | `hk install`                                                                                                                            |
| `lefthook uninstall`                        | `hk uninstall`                                                                                                                          |
| `lefthook run pre-commit`                   | `hk run pre-commit`                                                                                                                     |
| `lefthook run pre-commit --all-files`       | `hk run pre-commit --all`                                                                                                               |
| `lefthook run pre-commit --file a --file b` | `hk run pre-commit a b`                                                                                                                 |
| `lefthook run pre-commit --command eslint`  | `hk run pre-commit --step eslint`                                                                                                       |
| `lefthook run pre-commit --fail-on-changes` | `fail_on_fix = true` on the hook, with `stage = false`. See [review fixes before committing](/hooks#review-fixes-before-committing).    |
| `LEFTHOOK=0 git commit`                     | `HK=0 git commit`                                                                                                                       |
| `LEFTHOOK_EXCLUDE=eslint git commit`        | `HK_SKIP_STEPS=eslint git commit`                                                                                                       |

See [skip a hook or step](/hooks#skip-a-hook-or-step) for the rest.

## Example {#example}

A lefthook chart that formats and lints the staged cargo, checks the name ye christen her with, and runs tests before a push:

```yaml
pre-commit:
  parallel: true
  commands:
    prettier:
      glob: "*.{js,ts,css,md}"
      exclude: "dist/**"
      run: npx prettier --write {staged_files}
      stage_fixed: true
    eslint:
      glob: "*.{js,ts}"
      run: npx eslint --fix {staged_files}
      stage_fixed: true
commit-msg:
  commands:
    commitlint:
      run: npx commitlint --edit {1}
pre-push:
  commands:
    test:
      run: npm test
```

The same hooks in hk:

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.6.0/hk@2.6.0#/Config.pkl"

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["eslint"] {
        glob = "*.{js,ts}"
        fix = "npx eslint --fix {{files}}"
      }
      ["prettier"] {
        glob = "*.{js,ts,css,md}"
        exclude = "dist/**"
        depends = "eslint"
        fix = "npx prettier --write {{files}}"
      }
    }
  }
  ["commit-msg"] {
    steps {
      ["commitlint"] { check = "npx commitlint --edit {{commit_msg_file}}" }
    }
  }
  ["pre-push"] {
    steps {
      ["test"] { check = "npm test" }
    }
  }
}
```

- The lefthook chart ran `prettier` and `eslint` in parallel on the same files. `depends = "eslint"` makes hk run `eslint` first, so the two never rewrite a file at the same time. Drop it if the order does not matter; hk's file locks still prevent simultaneous writes.
- `pre-commit` stages what `eslint` and `prettier` changed. No `stage_fixed` is needed.
- `hk run pre-commit --plan` shows which steps and files a voyage would use.
