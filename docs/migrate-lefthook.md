---
description: Translate a lefthook.yml to hk.pkl by hand, with a mapping of options, placeholders, and commands, and the behavior differences to check.
---

# Migrating from lefthook

hk has no converter for lefthook. (`hk migrate pre-commit` handles only the pre-commit framework.) You translate `lefthook.yml` into `hk.pkl` by hand. The two tools share the same shape, hooks that contain named jobs that select files and run commands, so most configs map one to one.

## Differences to check first

- **Parallelism.** lefthook runs a hook's jobs in sequence unless you set `parallel: true`. hk runs steps concurrently by default, up to the job limit. It coordinates steps that select the same files with read and write locks, and you declare an order with `depends`. A lefthook config that relies on its sequence needs `depends` in hk. See [order steps deliberately](/hooks#order-steps-deliberately).
- **Check and fix.** A lefthook `run` is one command. hk separates `check`, which reports, from `fix`, which edits. A formatter that rewrites files in `pre-commit` becomes a `fix` command in a hook with `fix = true`.
- **Staging fixes.** lefthook stages changed files for jobs with `stage_fixed: true`. hk's `pre-commit` hook stages the files a step fixed by default, and a step's `stage` globs narrow or widen that. Other hooks leave fixes unstaged unless `stage = true`.
- **Partial commits.** `stash = "git"` on a hook makes hk set aside unstaged changes while it runs and restore them afterward, so linters see the staged content. See [stashing and partial commits](/hooks#stashing-and-partial-commits).
- **Failures.** hk stops after the first failing step by default (`fail_fast`). `hk run pre-commit --no-fail-fast` reports every failure.
- **Globs.** Both tools match a pattern without a slash against files at any depth, so `*.js` matches `a.js` and `src/b.js` in each. They differ for `**/*.js`: lefthook matches only files below a directory, and hk also matches `a.js` at the repository root.

## Move over

1. Write `hk.pkl` from your `lefthook.yml`, using the mapping below. `hk init` can generate a starting file for the tools it detects. See [getting started](/getting_started).
2. Run `hk validate`, then preview what a hook selects with `hk run pre-commit --plan`.
3. Remove lefthook's Git hooks with `lefthook uninstall`. That deletes the hooks it installed in `.git/hooks`. It leaves `lefthook.yml` unless you pass `--remove-configs`, so you can keep it for reference.
4. Run `hk install`. On Git 2.54 and newer hk registers its hooks in Git config, which runs alongside any other hook manager, so remove lefthook's hooks first or both run.
5. Delete `lefthook.yml`, `lefthook-local.yml`, and the lefthook dependency or tool pin.

## Hooks, jobs, and options

| lefthook                                          | hk                                                                                                                 |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `pre-commit`, `commit-msg`, `pre-push`            | A hook of the same name under `hooks`. See [other Git events](/hooks#other-git-events) for the others hk handles.   |
| `commands.<name>` or a `jobs` entry               | `steps { ["<name>"] { ... } }`                                                                                     |
| `run`                                             | `check` for a command that reports, `fix` for one that edits files                                                 |
| `glob`                                            | `glob`, a string or `List`. A `Regex` also works.                                                                  |
| `exclude`                                         | `exclude`                                                                                                          |
| `root`                                            | `dir`                                                                                                              |
| `env`                                             | `env` on the step, or on the hook for every step in it                                                             |
| `parallel: true`                                  | The default                                                                                                        |
| `piped: true`, `priority`                         | `depends` between steps                                                                                            |
| `stage_fixed: true`                               | The `pre-commit` default, or `stage` on the hook or step                                                           |
| `skip`, `only`                                    | `step_condition` or `condition`, or `profiles`. See [conditions](#conditions).                                     |
| `tags`                                            | `profiles`. A step with `profiles` runs only when you enable one of them with `--profile` or `HK_PROFILE`.         |
| `interactive: true`                               | `interactive = true`                                                                                               |
| `scripts`                                         | A step whose `check` or `fix` runs the script, for example `check = "./scripts/lint.sh {{files}}"`                 |
| `lefthook-local.yml`                              | `hk.local.pkl`, which amends `hk.pkl`. See [`hk.local.pkl`](/configuration#hk-local-pkl).                          |
| `extends`, `remotes`                              | Pkl `amends` and `import`. See [Pkl essentials](/pkl_introduction).                                                |
| `fail_text`                                       | No equivalent                                                                                                      |

## Placeholders

hk renders commands as [Tera](https://keats.github.io/tera/) templates, so a placeholder is written with double braces.

| lefthook                       | hk                                                                                                 |
| ------------------------------ | -------------------------------------------------------------------------------------------------- |
| `{staged_files}` in pre-commit | `{{files}}`, the files the hook selected that match the step. In `pre-commit` they are staged.      |
| `{push_files}` in pre-push     | `{{files}}`. In `pre-push` hk selects the files changed by the push.                                |
| `{1}` in commit-msg            | `{{commit_msg_file}}`                                                                              |
| `{1} {2}` in pre-push          | `{{hook_args}}`, the remote name and URL                                                           |
| `{all_files}`                  | No placeholder. Use a command that finds its own files, or run `hk run pre-commit --all`.          |

`{{files}}` is quoted for the shell and relative to the step's `dir`. See [define a step](/configuration#define-a-step) and [commit-message hooks](/hooks#commit-message-hooks).

## Conditions

lefthook's `skip: [merge]` has no built-in in hk. Write the condition yourself. `step_condition` is an expression, and `exec(...)` returns the command's standard output, untrimmed:

```pkl
["lint"] {
  // Skip while a merge is in progress
  step_condition = "exec('git rev-parse -q --verify MERGE_HEAD || true') == ''"
  check = "make lint"
}
["release-notes"] {
  // Run only on main
  step_condition = "trim(exec('git branch --show-current')) == 'main'"
  check = "make release-notes"
}
```

Make the command exit 0 and compare its output. If the command exits non-zero, hk reports that the condition could not be evaluated and the step still runs. See [conditions and Git status](/configuration#conditions-and-git-status).

## Commands and skipping

| lefthook                          | hk                                                                            |
| --------------------------------- | ----------------------------------------------------------------------------- |
| `lefthook install`                | `hk install`                                                                  |
| `lefthook uninstall`              | `hk uninstall`                                                                |
| `lefthook run pre-commit`         | `hk run pre-commit`                                                           |
| `lefthook run pre-commit --all-files` | `hk run pre-commit --all`                                                 |
| `lefthook run pre-commit --file a --file b` | `hk run pre-commit a b`                                             |
| `lefthook run pre-commit --command eslint` | `hk run pre-commit --step eslint`                                        |
| `lefthook run pre-commit --fail-on-changes` | `fail_on_fix = true` on the hook, with `stage = false`. See [review fixes before committing](/hooks#review-fixes-before-committing). |
| `LEFTHOOK=0 git commit`           | `HK=0 git commit`                                                             |
| `LEFTHOOK_EXCLUDE=eslint git commit` | `HK_SKIP_STEPS=eslint git commit`                                          |

See [skip a hook or step](/hooks#skip-a-hook-or-step) for the rest.

## Example

A lefthook config that formats and lints staged files, checks the commit message, and runs tests before a push:

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
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"

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

- The lefthook config ran `prettier` and `eslint` in parallel on the same files. `depends = "eslint"` makes hk run `eslint` first, so the two never rewrite a file at the same time. Drop it if the order does not matter; hk's file locks still prevent simultaneous writes.
- `pre-commit` stages what `eslint` and `prettier` changed. No `stage_fixed` is needed.
- `hk run pre-commit --plan` shows which steps and files a commit would use.
