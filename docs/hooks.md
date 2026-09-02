# Hooks

This page describes the behavior of the git hooks that hk supports. Each step provides a "check" and a "fix" command. "check" commands are read-only and can run in parallel. "fix" commands can edit files and block other "fix" or "check" commands on the same files from running at the same time. For performance reasons, hk does not enforce that "check" commands leave files untouched, so follow this convention yourself for hk to behave as expected.

This read/write locking is what lets hk run hooks as fast as possible while staying safe.

## Hook Behavior

With `fix = true`, an hk hook performs the following:

* Stashes any untracked/unstaged changes if stashing is enabled. It is off by default; see [`HK_STASH`](/environment_variables#hk-stash)
* Gathers the list of files with staged changes (or all files if running `hk run pre-commit --all`)
* Runs linters and hook steps in parallel up to [`HK_JOBS`](/environment_variables#hk-jobs) at a time, with caveats:
  * `exclusive = true` steps wait until all previous steps have finished and block later steps from starting
  * if a step has dependencies, hk waits for them to complete before starting it
  * hk takes read/write locks on each file the step matches (according to its glob patterns) unless `stomp = true`
  * if `check_first = true` on the step, hk runs the "check" command first with read locks; if that fails, it runs the "fix" command with write locks on all the files
  * if the step has a `check_list_files` command, hk uses its output to narrow the files it takes write locks on and passes to "fix"
  * if `check_first = false` on the step, hk runs the "fix" command after taking write locks, blocking other steps on the same files. Avoid this configuration for performance reasons.
  * if any files were modified and match the `stage` globs, they are added to the git index (`stage` defaults to the step's `glob` for steps with a `fix` command)
* Restores the stashed untracked/unstaged changes

If `fix = false`, hk only runs the `check` commands and does not need read/write locks, since nothing should be making modifications. Steps with [`check_failed_files = true`](/configuration#focus-checks-on-failing-files) first use `check_diff` or `check_list_files` to identify affected paths, then run the detailed `check` command only on that focused set.

### Allowing a step to fail

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

## `pre-commit`

Runs during `git commit`, before the commit is created.

```pkl
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
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

Runs during `git commit`, before the commit message editor opens. Useful for rendering a default commit message template.
The `commit_msg_file`, `source`, and `sha` template variables are available in this hook. The raw git hook arguments are also available as `hook_args`.

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

Runs during `git commit`, after the commit message has been written. Useful for validating the commit message.
The `commit_msg_file` template variable is available in this hook. The raw git hook arguments are also available as `hook_args`.

```pkl
hooks {
    ["commit-msg"] {
        steps {
            ["validate-commit-msg"] {
                check = "grep -Eq '^(fix|feat|chore):' {{commit_msg_file}}"
            }
        }
    }
}
```

## `post-checkout`

Runs after `git checkout` updates the worktree. The `prev_head`, `new_head`, and `is_branch_checkout` template variables are available in this hook. `is_branch_checkout` is a boolean value. The raw git hook arguments are also available as `hook_args`.

```pkl
hooks {
    ["post-checkout"] {
        steps {
            ["restore-lfs"] {
                check = "git lfs post-checkout {{ hook_args }}"
            }
        }
    }
}
```

## Other Hooks

Other git hooks are also supported. See <https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks>.
The raw arguments for hooks with dedicated handlers are available as `hook_args`; hooks without dedicated handlers get an empty `hook_args`.
