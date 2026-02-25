# Hooks

The following describes the behavior of git hooks that hk supports. Each linter provides a "check" and "fix" commands. "check" commands are read-only and can be run in parallel. "fix" commands can edit files and will block other "fix" or "check" commands from running at the same time. Note that hk does not enforce that "check" commands do not write to files for performance reasons however you still should try to follow this convention in order for hk to behave as expected.

It's the read/write locking behavior that hk makes use of in order to run hooks as fast as possible while still being safe.

## Hook Behavior

hk hooks perform the following assuming `fix = true`:

* Stashes any untracked/unstaged changes (disable with [`HK_STASH=none`](/configuration#hk-stash))
* Gathers list of files with staged changes (or all files if running `hk run pre-commit --all`)
* Runs linters and hook steps in parallel up to [`HK_JOBS`](/configuration#hk-jobs) at a time, with caveats:
  * `exclusive = true` hook steps will wait until all previous steps finished and block later steps from starting
  * if any hook step has any dependencies, hk will wait for them to complete before starting
  * hk will create read/write locks for each file (according to the linter's glob patterns) to check/fix in the linters unless `stomp = true`
  * if `check_first = true` on the linter, hk will run the "check" command first with read locks, if that fails, it will run the "fix" command with write locks on all the files
  * if a `check_list_files` command is available on the linter, hk will use the output of that command to filter the list of files to get write locks for and call "fix" on.
  * if `check_first = false` on the linter, hk will run the "fix" command after fetching write locks, blocking other linters from running. You
    should avoid this performance.
  * if any of the files have been modified and match the `stage` globs, they will be added to the git index (defaults to the step's `glob` for steps with a `fix` command)
* untracked/unstaged changes are unstashed

If `fix = false`, hk will just run the `check` steps and won't need to deal with read/write locks as nothing should be making modifications.

## `pre-commit`

Runs when `git commit` is run before `git commit` creates the commit.

```pkl
hooks {
    fix = true
    ["pre-commit"] {
        steps {
            ["cargo-fmt"] {
                glob = "*.rs"
                check_first = true
                check = "cargo fmt --check"
                fix = "cargo fmt"
            }
            ["cargo-clippy"] {
                glob = "*.rs"
                check_first = true
                check = "cargo clippy"
                fix = "cargo clippy --fix --allow-dirty --allow-staged"
            }
        }
    }
}
```

## `prepare-commit-msg`

Runs when `git commit` is run before the commit message is created. Useful for rendering a default commit message template.

```pkl
hooks {
    ["prepare-commit-msg"] {
        steps {
            ["render-commit-msg"] {
                check = "echo 'default commit message' > {{commit_msg_file}}"
            }
        }
    }
}

```

## `commit-msg`

Runs when `git commit` is run after the commit message is created. Useful for validating the commit message.

```pkl
hooks {
    ["commit-msg"] {
        steps {
            ["validate-commit-msg"] {
                check = "grep -q '^(fix|feat|chore):' {{commit_msg_file}} || exit 1"
            }
        }
    }
}
```

## Other Hooks

Other git hooks are also supported. See <https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks>.
