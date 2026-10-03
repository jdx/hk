---
description: Replace husky and lint-staged with hk by hand, with a mapping of hooks, globs, and commands, and the behavior differences to check.
---

# Migrating from husky and lint-staged

husky runs the scripts in `.husky/` as Git hooks. lint-staged picks the staged files that match each glob and runs your commands on them, usually from a husky `pre-commit` script. hk does both jobs: its hooks select files, run steps, and can stage fixes and protect unstaged work.

hk has no converter for these tools. (`hk migrate pre-commit` handles only the pre-commit framework.) You translate by hand, and the mapping is direct: a lint-staged glob becomes a step's `glob`, its commands become `fix` or `check`, and a husky script becomes a hook.

## Differences to check first

- **Local binaries.** husky adds `node_modules/.bin` to `PATH` for its scripts, and lint-staged finds locally installed tools. hk runs commands with the `PATH` it was started with. Call project tools with `npx` or your package manager's exec command, set `prefix = "npx"` on a step, or put the tools on `PATH`. The [mise integration guide](/mise_integration) shows a Node.js setup.
- **Concurrency.** lint-staged runs the commands for one glob in sequence and runs the globs concurrently. hk runs steps concurrently and coordinates steps that select the same files with read and write locks. Put commands that must run in order in one step joined with `&&`, or in separate steps linked by `depends`. See [order steps deliberately](/hooks#order-steps-deliberately).
- **Unstaged changes.** lint-staged backs up your state in a Git stash and sets aside unstaged changes in partially staged files while its tasks run, unless you pass `--no-stash`. hk does this when a hook sets `stash = "git"`; with no `stash` setting it leaves unstaged work in place. See [stashing and partial commits](/hooks#stashing-and-partial-commits).
- **Staging fixes.** lint-staged stages what its tasks changed. hk's `pre-commit` hook does the same for steps that fix, by default. `stage = false` with `fail_on_fix = true` stops the commit for review instead, like lint-staged's `--fail-on-changes`. See [review fixes before committing](/hooks#review-fixes-before-committing).
- **Paths.** lint-staged passes absolute paths unless you set `--relative`. hk's `{{files}}` are relative to the repository, or to the step's `dir`.
- **Failures.** Both stop after the first failure by default. `hk run pre-commit --no-fail-fast` reports every failure, like `--continue-on-error`.
- **Globs.** Both match a pattern without a slash, such as `*.js`, against files at any depth. lint-staged negation patterns such as `!(*.test).js` become an `exclude` list.
- **No function configs.** lint-staged accepts JavaScript functions that build commands. hk steps are strings, so build the command with the `{{files}}` placeholder and the other [template variables](/configuration#define-a-step).

## Move over

1. Write `hk.pkl` from your lint-staged config and husky scripts, using the mappings below. `hk init` can generate a starting file for the tools it detects. See [getting started](/getting_started).
2. Run `hk validate`, then preview what a hook selects with `hk run pre-commit --plan`.
3. Remove husky:
   - Delete the `prepare` script (`"prepare": "husky"`) from `package.json`. husky sets `core.hooksPath` to `.husky/_` each time it runs, so a leftover `prepare` brings it back.
   - Uninstall `husky` and `lint-staged` with your package manager and delete `.husky/`.
   - Run `git config --unset core.hooksPath`.
4. Run `hk install`. On Git 2.54 and newer hk registers its hooks in Git config, which runs alongside hooks from `core.hooksPath`, so both husky and hk run until you complete step 3. On older Git, hk writes hook files to the hooks directory, and a `core.hooksPath` that points elsewhere can stop them from running; `hk install` warns when it finds `core.hooksPath` set.

Everyone who has a clone must run `hk install`, and must unset `core.hooksPath` if it still points at `.husky/_`.

## Hooks and scripts

| husky                                              | hk                                                                                                              |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `.husky/pre-commit`, `commit-msg`, `pre-push`      | A hook of the same name under `hooks`. See [other Git events](/hooks#other-git-events) for the others hk handles. |
| `npx lint-staged` in `pre-commit`                  | The steps of the `pre-commit` hook, one per lint-staged glob                                                    |
| `npx --no -- commitlint --edit $1` in `commit-msg` | `check = "npx commitlint --edit {{commit_msg_file}}"`. See [commit-message hooks](/hooks#commit-message-hooks). |
| `npm test` in `pre-push`                           | A `pre-push` step with `check = "npm test"` and no `glob`, so it always runs                                    |
| `HUSKY=0 git commit`                               | `HK=0 git commit`                                                                                               |

A `pre-push` hook also receives the remote name and URL as `{{hook_args}}`. A step without a `glob` runs even when no files match.

## lint-staged options

| lint-staged                                | hk                                                                                          |
| ------------------------------------------ | ------------------------------------------------------------------------------------------- |
| A glob key such as `"*.{js,ts}"`           | `glob = "*.{js,ts}"` on a step. A `List` or a `Regex` also works.                           |
| A command string                           | `fix` for a command that edits files, `check` for one that only reports                     |
| Files appended to the command              | `{{files}}`, written where the files belong                                                 |
| A list of commands for one glob            | One step with the commands joined by `&&`, or one step each linked by `depends`             |
| `!` negation                               | `exclude`                                                                                   |
| `--no-stash`                               | `stash = "none"`, or leave `stash` unset                                                    |
| Default backup and restore                 | `stash = "git"` on the hook                                                                 |
| `--fail-on-changes`                        | `fail_on_fix = true` and `stage = false` on the hook                                        |
| `--continue-on-error`                      | `--no-fail-fast`, or `fail_fast = false`                                                    |
| `--concurrent <n>`                         | `--jobs <n>` or `jobs = n`                                                                  |
| Per-directory configs in a monorepo        | `subprojects`. See [monorepos](/configuration#subprojects).                   |

## Example

A husky and lint-staged setup. In `package.json`:

```json
{
  "scripts": { "prepare": "husky" },
  "lint-staged": {
    "*.{js,ts}": ["eslint --fix", "prettier --write"],
    "*.css": "stylelint --fix",
    "*.md": "prettier --write"
  }
}
```

In `.husky/pre-commit`:

```sh
npx lint-staged
```

In `.husky/commit-msg`:

```sh
npx --no -- commitlint --edit $1
```

In `.husky/pre-push`:

```sh
npm test
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
        glob = "*.{js,ts,md}"
        depends = "eslint"
        fix = "npx prettier --write {{files}}"
      }
      ["stylelint"] {
        glob = "*.css"
        fix = "npx stylelint --fix {{files}}"
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

- lint-staged ran `eslint --fix` and then `prettier --write` on JavaScript and TypeScript files. `depends = "eslint"` keeps that order. The `prettier` step also covers Markdown, which lint-staged handled with a separate key.
- `stylelint` has no dependency, so hk runs it alongside `eslint`.
- `hk run pre-commit --plan` shows which steps and files a commit would use.
