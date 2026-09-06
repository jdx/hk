---
description: Configure Git hooks, understand staged-file selection, and control fixes, stashing, and execution order.
---

# Git hooks and stashing

A hook is a named collection of steps. Git invokes installed hooks at specific events; `hk run <hook>` invokes them directly. The `check` and `fix` hooks are also available through `hk check` and `hk fix`.

For installation, see [getting started](/getting_started#install-hooks).

## Check and fix commands

A step’s `check` command should report problems without editing files. Its `fix` command may edit them. A hook with `fix = true` uses the fix workflow; steps with only a check command can still validate files.

hk uses read locks for checks and write locks for fixes. These locks coordinate steps that select the same files. hk does not sandbox commands or detect every undeclared write, so a check that edits files can interfere with other steps.

Use [structured output and command effects](/agents) when integrating checks with coding agents or automation.

## File selection

With the generated configuration:

| Invocation                               | Default selection                                       |
| ---------------------------------------- | ------------------------------------------------------- |
| `hk run pre-commit`                      | Staged files                                            |
| `hk check` / `hk fix`                    | Modified files: staged, unstaged, and untracked         |
| `hk check --all`                         | Tracked files and eligible untracked files              |
| `hk check --staged`                      | Staged paths, using their current working-tree contents |
| `hk check --from-ref main --to-ref HEAD` | Paths changed between those references                  |

Step patterns, exclusions, ignore rules, and settings further filter this selection. Enabling stashing also changes default selection to staged files.

For both `hk run pre-commit --all` and `hk check --all`, the resolved stash method controls untracked-file selection: `git` and `patch-file` select tracked files only; `none` also selects discovered untracked files. When `--stash` is omitted, `HK_STASH` overrides the hook setting.

`HK_STASH_UNTRACKED=0` disables discovery and stashing of untracked files. Setting it to `1` allows discovery and stashing, but does not include untracked files in `--all` when stashing is enabled.

::: warning Staged paths are not staged contents
`--staged` does not stash unstaged changes. If a file contains both staged and unstaged edits, the command sees its working-tree contents. Use a hook configured with stashing when the staged version must be isolated.
:::

Use `hk run pre-commit --plan` to inspect selected steps and files before executing them.

## Stashing and partial commits

For a pre-commit hook that applies fixes, set both `fix` and `stash`:

```pkl
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = linters
  }
}
```

hk saves unstaged work, runs the hook against the staged content, stages applicable fixes, and restores the saved work. This lets you use `git add -p` without intentionally including the rest of your edits.

Linters still operate on whole files. Staging one hunk does not restrict a formatter to that hunk.

### Choose a stash strategy

| Value               | Behavior                                            |
| ------------------- | --------------------------------------------------- |
| `"git"` or `true`   | Save unstaged changes with Git stashing             |
| `"patch-file"`      | Currently an alias for the Git stash implementation |
| `"none"` or `false` | Leave unstaged work in place                        |

An unspecified hook stash setting defaults to `"none"`. `hk init` explicitly enables `"git"` for pre-commit. Override a run with `--stash` or [`HK_STASH`](/environment_variables#hk-stash).

Untracked files are included in stashing by default. `HK_STASH_UNTRACKED=0` also disables their discovery, which can help very large worktrees but changes file selection.

### If restoration fails

Read hk’s error before changing the working tree. Inspect `git status`, `git diff`, `git diff --cached`, and `git stash list` to understand which changes are present.

hk keeps backup patches under `$HK_STATE_DIR/patches/` when Git stashing is used; the `stash_backup_count` setting controls retention. Preserve the reported stash and backup until you have recovered and reviewed your work. Avoid blindly applying a stash again to files that already contain its changes.

## Review fixes before committing

The generated pre-commit hook stages applicable fixes automatically. To apply fixes but stop the commit for review:

```pkl
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    stage = false
    fail_on_fix = true
    steps = linters
  }
}
```

When a fixer changes a file, hk fails the hook and leaves the fixes for you to review and stage. Retry the commit afterward. `stage = false` alone disables staging without requiring the hook to fail.

For a single run, use `--no-stage` to disable automatic staging.

<span id="hook-behavior"></span>

## Order steps deliberately

Steps run concurrently, up to the job limit, unless coordination requires them to wait:

- `depends = "eslint"` waits for the named step.
- `exclusive = true` waits for earlier steps and blocks later ones until it finishes.
- A `Group` creates a boundary: its children run together, and later groups wait.
- Read/write locks coordinate steps that select overlapping files.

Locks prevent simultaneous writes; they do not choose the final style when tools disagree. Configure compatible rules or declare an explicit dependency.

## Allowing a step to fail

Set `allow_failure = true` on a step to run it and report a non-zero command
exit without failing the hook. This is narrower than bypassing the hook or
skipping the step: all other steps retain their normal blocking behavior, and
errors from hk itself are still fatal.

The setting can be an expression using `env(name)` when the policy should be
conditional on an environment variable:

```pkl
["cargo-check"] {
    check = "cargo check"
    allow_failure = "env('KNOWN_BROKEN') == 'true'"
}
```

`KNOWN_BROKEN=true git commit` will therefore show a failed `cargo-check` but
allow the commit, while an ordinary `git commit` remains blocked by the same
failure.

## Commit-message hooks

`commit-msg` runs after the message is prepared and before the commit is created. Use the built-in Conventional Commits check:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Builtins.pkl"

hooks {
  ["commit-msg"] {
    steps {
      ["conventional-commit"] = Builtins.check_conventional_commit
    }
  }
}
```

The `commit_msg_file` template variable contains the message path. `prepare-commit-msg` also receives `source` and `sha` when Git supplies them. Use that hook to prepare or edit a message before the user’s editor opens.

## Other Git events

hk has dedicated handlers for these events:

| Event                | Useful template variables                                 |
| -------------------- | --------------------------------------------------------- |
| `pre-commit`         | Staged file selection                                     |
| `pre-push`           | `hook_args` (remote and URL), `hook_stdin` (updated refs) |
| `commit-msg`         | `commit_msg_file`                                         |
| `prepare-commit-msg` | `commit_msg_file`, `source`, `sha`                        |
| `post-checkout`      | `prev_head`, `new_head`, `is_branch_checkout`             |
| `post-merge`         | `hook_args` (squash flag)                                 |
| `post-rewrite`       | `hook_args` (command), `hook_stdin` (rewritten refs)      |
| `pre-rebase`         | `hook_args` (upstream and optional branch)                |
| `post-commit`        | No event-specific arguments                               |

Dedicated handlers also expose their raw arguments as `hook_args`. See the [run reference](/cli/run) for argument details. Custom hooks can be invoked by name; hooks without a dedicated handler receive an empty `hook_args` value.

## Skip a hook or step

```sh
HK_SKIP_STEPS=eslint git commit
HK_SKIP_HOOK=pre-push git push
HK=0 git commit
```

`HK=0` bypasses hk’s installed hook launcher. To persist a preference, use [Git configuration](/configuration#git-configuration), such as `git config --local hk.skipSteps eslint`.
