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

A hook defaults to `"none"`, with one exception: when the configuration has top-level `steps` (the setup `hk init` creates), hk fills in `"git"` for `pre-commit`, including a `pre-commit` hook you define explicitly without a `stash` setting. Set `stash = "none"` on the hook to leave unstaged work in place. Override a run with `--stash` or [`HK_STASH`](/environment_variables#hk-stash).

Untracked files are included in stashing by default. `HK_STASH_UNTRACKED=0` also disables their discovery, which can help very large worktrees but changes file selection.

Files marked with `git add -N` (intent to add) have no staged content, so hk sets them aside as well, even with `HK_STASH_UNTRACKED=0`. Steps do not receive them, and afterward hk restores them and marks them intent-to-add again.

### If restoration fails

Read hk’s error before changing the working tree. Inspect `git status`, `git diff`, `git diff --cached`, and `git stash list` to understand which changes are present.

hk keeps backup patches under `$HK_STATE_DIR/patches/` when Git stashing is used; the `stash_backup_count` setting controls retention. Preserve the reported stash and backup until you have recovered and reviewed your work. Avoid blindly applying a stash again to files that already contain its changes.

While hk has your unstaged changes set aside, it holds a lock file, `hk-stash.lock`, in the repository's common git directory, which linked worktrees share. Two hk processes in one repository, such as hooks in two worktrees, therefore take turns stashing and restoring instead of restoring each other's changes. The one that waits prints a message and fails after [`HK_STASH_LOCK_TIMEOUT`](/environment_variables#hk-stash-lock-timeout) seconds, naming the lock file. The operating system releases the lock if hk is killed.

Intent-to-add files are kept in a separate stash entry whose message ends with `(intent-to-add files)`, such as `hk: 4242-1a2b-3 (intent-to-add files)`; hk versions before this one named it `hk: intent-to-add files`. To recover them, run `git stash apply` on that entry, then `git add -N` the files again.

### If hk is stopped mid-run {#if-hk-is-stopped-mid-run}

While hk has your unstaged changes stashed, it keeps a small journal named `hk-pending-stash` in the repository's git directory (`git rev-parse --git-dir`, so each linked worktree has its own). hk writes it before it touches the working tree and deletes it once your changes are back. It records the stash commit ids, the process id, the hook, the time, and the host name and (on Linux) PID namespace it ran in. Every change to it, and every recovery, happens while hk holds a lock on a second file next to it, `hk-pending-stash.lock`, so two hk runs never act on it at once; the operating system releases that lock if hk dies, and hk never deletes the file, which holds no data, so leave it in place (removing it while a run is recovering would let a second run lock a fresh copy and recover the same journal at the same time). In a repository with `core.sharedRepository` set (`group`, `all` or an octal mode), hk gives the journal, its lock file and the temporary file it writes it through the same permissions git gives its own files, whatever the umask, so another account of the group can read a journal a killed run left and recover it; without it the umask applies. A journal hk cannot read (for example without permission) is never taken for absent: hk warns, naming the file, and says to look in `git stash list`.

These are two different locks: `hk-stash.lock` (above, in the common git directory) keeps hk processes from stashing at the same time and is held from before the stash is made until it is restored, while `hk-pending-stash.lock` only guards the journal file. hk takes the stash lock first, then writes the journal, restores your changes, removes the journal and releases the stash lock. Recovery takes the locks in the same order: the stash lock, then the journal lock, and only then does it look at the working tree, so no other hk can stash, restore or recover in between. If the stash lock is not free within `HK_STASH_LOCK_TIMEOUT`, hk prints a warning, leaves the journal untouched and carries on; the next run tries again. A signal while hk is still waiting for the stash lock stops the wait; nothing was stashed, so no journal is written and nothing is restored.

- **SIGINT, SIGTERM, SIGHUP** (and Ctrl+C or Ctrl+Break on Windows): hk stops the running steps, puts your changes back, removes the journal and exits. SIGTERM and SIGHUP exit with `128` plus the signal number (`143` and `129`); Ctrl+C exits with the status the run ended with, `1` for a cancelled run. It gives itself 10 seconds; sending the same signal twice exits at once. Restoring never depends on the terminal: when the terminal is gone (SIGHUP), hk sends its output to the null device, so restoring still works.
- **Closing the console, logging off or shutting down Windows**: hk handles these like SIGHUP (exit status `129`, output sent to `NUL`) and starts restoring at once. Windows ends the process a few seconds after these events, which can be before the restore finishes. When it is, the journal stays and the next hk run recovers it, as after a crash.
- **SIGKILL, a crash or a power cut**: nothing can run, so your changes stay in the stash and the journal stays. The next hk run in that repository (any hook, `check`, `fix` or `run`) reads it before it stashes anything. If the process that wrote it is gone, hk puts the changes back when the working tree has no unstaged changes and no untracked files. Otherwise, or when it cannot tell which stash entries are its own, it changes nothing and prints the exact `git stash apply <commit-id>` command to run. A journal whose process is still running is never touched, and hk says so when it names stashed changes. A process id that is in use but whose start time hk cannot read (a process it may not inspect) is not taken for the owner either: it is handled as a journal whose owner cannot be told, below.

A process id only means something where it was written. A repository's git directory can be shared by several hosts (on a network file system) or PID namespaces (a repository mounted into a container), which cannot see each other's processes. hk therefore treats the process as gone only when the journal's recorded host name and PID namespace are the current ones and the process is not running. When they differ, when hk cannot read them, or when the journal was written by an older hk that did not record them and its process id is not running here, hk cannot know whether the run is still going. It then restores and drops nothing, prints the `git stash apply <commit-id>` command for each entry the journal names and for each entry named `hk: <pid>-...` after its process id that the journal never recorded (hk stopped between stashing and recording), and says the journal was written elsewhere; if it cannot read the stash list it says to look for those entries with `git stash list`; if that run is still going it finishes and removes its own journal. A journal like this that names no entry still in `git stash list` protects nothing, so hk removes it with a notice and later runs write journals of their own. On macOS and Windows only the host name is compared.

hk reminds you on every run until the stash entry is gone: once you have applied it and dropped it, hk removes the journal itself, or you can delete the file. If another hk's journal is already in place, a run does not write its own and warns that it keeps none; if it is killed, its stash entry stays in `git stash list` as `hk: <pid>-...`. Because a run holds the stash lock from before it stashes until it restores, no other run can recover the older journal in between, so an old stash is never put back over a fixer that is still running.

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
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Builtins.pkl"

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

These variables are also available in `condition` and `step_condition` expressions. For example, to run a `post-checkout` step only on branch checkouts and report it as skipped on file checkouts:

```pkl
hooks {
  ["post-checkout"] {
    steps {
      ["install-deps"] {
        step_condition = "is_branch_checkout"
        check = "mise install"
      }
    }
  }
}
```

## Skip a hook or step

```sh
HK_SKIP_STEPS=eslint git commit
HK_SKIP_HOOK=pre-push git push
HK=0 git commit
```

`HK=0` bypasses hk’s installed hook launcher. To persist a preference, use [Git configuration](/configuration#git-configuration), such as `git config --local hk.skipSteps eslint`.
