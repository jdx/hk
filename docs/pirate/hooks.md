---
description: Rig yer Git hooks, learn how hk picks the staged cargo, and command the fixes, the stowing of the hold, and the order the hands work in.
sourceHash: 519685343fcf
---

# Git hooks and stowing the hold

A hook is a named muster of steps: the bosun's pipe that calls a set of hands on deck. Git sounds the installed hooks when particular events come round; `hk run <hook>` sounds them directly. The `check` and `fix` hooks also answer to `hk check` and `hk fix`.

To rig the hooks, see [getting started](/getting_started#install-hooks).

## Check and fix: inspecting the cargo, mending the canvas {#check-and-fix-commands}

A step's `check` command should report problems without editing files: a lookout looks and touches nothing. Its `fix` command may edit them. A hook with `fix = true` works the fix workflow; steps with only a check command can still inspect the cargo.

hk takes read locks for checks and write locks for fixes. These lashings coordinate the hands that pick the same files. But hk does not sandbox commands or catch every write a step never declared, so a check that edits files can interfere with the other hands.

Use [structured output and command effects](/agents) when ye wire checks into clockwork hands (coding agents) or other automation.

## Which cargo gets picked {#file-selection}

With the generated configuration:

| The call                                 | The cargo it picks by default                               |
| ---------------------------------------- | ----------------------------------------------------------- |
| `hk run pre-commit`                      | Staged files, the cargo loaded aboard for this voyage       |
| `hk check` / `hk fix`                    | Modified files: staged, unstaged, and untracked             |
| `hk check --all`                         | Tracked files and eligible untracked files                  |
| `hk check --staged`                      | Staged paths, read with their current working-tree contents |
| `hk check --from-ref main --to-ref HEAD` | Paths changed between those references                      |

Step patterns, exclusions, ignore rules, and settings sift this selection further. Turning stashing on also changes the default selection to staged files.

For both `hk run pre-commit --all` and `hk check --all`, the resolved stash method decides whether untracked files are picked: `git` and `patch-file` pick tracked files only; `none` also picks the untracked files hk discovers. When ye leave `--stash` off, `HK_STASH` overrides the hook's setting.

`HK_STASH_UNTRACKED=0` stops hk discovering untracked files or stowing them in the hold. Setting it to `1` allows discovery and stashing, but still leaves untracked files out of `--all` when stashing is on.

::: warning Mind ye: staged paths are not staged contents
`--staged` does not stow unstaged changes in the hold. If a file carries both staged and unstaged edits, the command sees its working-tree contents, dock cargo and all. Use a hook set up with stashing when the staged version must be kept apart on its own.
:::

Use `hk run pre-commit --plan` to read the passage plan: the steps and files it picks, before any of them run.

## Stowing the hold for a partial voyage {#stashing-and-partial-commits}

For a pre-commit hook that mends the canvas, set both `fix` and `stash`:

```pkl
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = linters
  }
}
```

hk stows yer unstaged work in the hold, runs the hook against the staged content, stages the fixes that apply, and brings the stowed work back up. So ye can load cargo with `git add -p` without having to take the rest of yer edits aboard. As the song has it, yer line lies snug in the hold.

Linters still work on whole files. Loading one hunk aboard does not keep a formatter to that hunk.

### Choose how to stow {#choose-a-stash-strategy}

| Value               | What the crew does                                     |
| ------------------- | ------------------------------------------------------ |
| `"git"` or `true`   | Stow unstaged changes in the hold with Git stashing    |
| `"patch-file"`      | For now, another name for the Git stash implementation |
| `"none"` or `false` | Leave unstaged work where it lies                      |

A hook defaults to `"none"`, with one exception: when the config has top-level `steps` (the setup `hk init` creates), hk fills in `"git"` for `pre-commit`, even a `pre-commit` hook ye define yerself with no stash setting. Set `stash = "none"` on the hook to leave unstaged work where it lies. To change course for a single run, use `--stash` or [`HK_STASH`](/environment_variables#hk-stash).

Untracked files are stowed along with the rest by default. `HK_STASH_UNTRACKED=0` also stops hk discovering them, which can help on very large worktrees but changes which files get picked.

Files marked with `git add -N` (intent to add) have no staged content, so hk sets them aside as well, even with `HK_STASH_UNTRACKED=0`. Steps don't receive them, and afterward hk brings them back up and marks them intent-to-add again.

### If the stowed work won't come up {#if-restoration-fails}

Read hk's error before ye change the working tree. Inspect `git status`, `git diff`, `git diff --cached`, and `git stash list` to see which changes are present.

hk keeps backup patches under `$HK_STATE_DIR/patches/` when Git stashing is used; the `stash_backup_count` setting controls how many it keeps. Keep the stash and backup hk reports until ye have recovered and reviewed yer work. Don't blindly apply a stash again to files that already carry its changes.

While hk has yer unstaged changes stowed, it holds a lock file, `hk-stash.lock`, in the repository's common git directory, which linked worktrees share. Two hk processes in one repository, such as hooks in two worktrees, therefore take turns stowing and restoring instead of hauling up each other's cargo. The one that waits prints a message and gives up after [`HK_STASH_LOCK_TIMEOUT`](/environment_variables#hk-stash-lock-timeout) seconds, naming the lock file. The operating system drops the lock if hk is sunk.

Intent-to-add files are kept in a separate stash entry whose message ends with `(intent-to-add files)`, such as `hk: 4242-1a2b-3 (intent-to-add files)`; hk versions before this one named it `hk: intent-to-add files`. To recover them, run `git stash apply` on that entry, then `git add -N` the files again.

### If hk is stopped mid-run {#if-hk-is-stopped-mid-run}

While hk has yer unstaged changes stowed in the hold, it keeps a small journal named `hk-pending-stash` in the repository's git directory (`git rev-parse --git-dir`, so each linked worktree has its own). hk writes it before it touches the working tree and strikes it once yer changes are back up. It records the stash commit ids, the process id, the hook, the time, and the host name and (on Linux) PID namespace it sailed in. Every change to it, and every recovery, happens while hk holds a lock on a second file beside it, `hk-pending-stash.lock`, so two hk runs never lay hands on it at once; the operating system lets go of that lock if hk dies, and hk never strikes the file, which holds no data, so leave it where it lies (removing it while a run is recovering would let a second run lock a fresh copy and recover the same journal at the same time). In a repository with `core.sharedRepository` set (`group`, `all` or an octal mode), hk gives the journal, its lock file and the temporary file it writes it through the same permissions git gives its own files, whatever the umask, so another account of the group can read a journal a killed run left and recover it; without it the umask applies. A journal hk cannot read (without permission, say) is never taken for absent: hk warns, naming the file, and bids ye look in `git stash list`.

These be two different locks: `hk-stash.lock` (above, in the common git directory) keeps hk processes from stowing at the same time and is held from before the hold is filled until it is emptied again, while `hk-pending-stash.lock` only guards the journal file. hk takes the stash lock first, then writes the journal, brings yer changes back up, strikes the journal and lets go of the stash lock. Recovery takes the locks in the same order: the stash lock, then the journal lock, and only then does it look at the working tree, so no other hk can stow, restore or recover in between. If the stash lock is not free within `HK_STASH_LOCK_TIMEOUT`, hk prints a warning, leaves the journal untouched and sails on; the next run tries again. A signal while hk is still waiting for the stash lock ends the wait; nothing was stowed, so no journal is written and nothing is brought back up.

- **SIGINT, SIGTERM, SIGHUP** (and Ctrl+C or Ctrl+Break on Windows): hk stops the running hands, brings yer changes back up, removes the journal and exits. SIGTERM and SIGHUP exit with `128` plus the signal number (`143` and `129`); Ctrl+C exits with the status the run ended with, `1` for a cancelled run. It gives itself 10 seconds; sending the same signal twice exits at once. Restoring never depends on the terminal: when the terminal is lost overboard (SIGHUP), hk sends its output to the null device, so the restoring carries on.
- **Closing the console, logging off or shutting down Windows**: hk handles these like SIGHUP (exit status `129`, output sent to `NUL`) and starts restoring at once. Windows ends the process a few seconds after these events, which can be before the restore finishes. When it is, the journal stays and the next hk run recovers it, as after a crash.
- **SIGKILL, a crash or a power cut**: nothing can run, so yer changes stay in the hold and the journal stays. The next hk run in that repository (any hook, `check`, `fix` or `run`) reads it before it stows anything. If the process that wrote it is gone, hk brings the changes back up when the working tree has no unstaged changes and no untracked files. Otherwise, or when it cannot tell which stash entries are its own, it changes nothing and prints the exact `git stash apply <commit-id>` command to run. A journal whose process is still running is never touched, and hk says so when it names stowed changes. A process id that be in use but whose start time hk cannot read (a vessel it may not board) is not taken for the owner either: it be handled as a journal whose owner cannot be told, below.

A process id only means something where it was written. A repository's git directory can be shared by several hosts (on a network file system) or PID namespaces (a repository mounted into a container), which cannot see each other's processes. hk therefore treats the process as gone only when the journal's recorded host name and PID namespace are the current ones and the process is not running. When they differ, when hk cannot read them, or when the journal was written by an older hk that did not record them and its process id is not running here, hk cannot know whether the run is still afloat. It then restores and drops nothing, prints the `git stash apply <commit-id>` command for each entry the journal names and for each entry named `hk: <pid>-...` after its process id that the journal never recorded (hk stopped between stowing and recording), and says the journal was written elsewhere; if it cannot read the hold's list it bids ye look for those entries with `git stash list`; if that run is still afloat it finishes and strikes its own journal. A journal like this that names no entry still in `git stash list` guards nothing, so hk strikes it with a notice and later runs write journals of their own. On macOS and Windows only the host name is compared.

hk reminds ye on every run until the stash entry is gone: once ye have applied it and dropped it, hk removes the journal itself, or ye can delete the file. If another hk's journal is already in place, a run does not write its own and warns that it keeps none; if it is sunk, its stash entry stays in `git stash list` as `hk: <pid>-...`. Because a run holds the stash lock from before it stows until it restores, no other run can recover the older journal in between, so an old stash is never put back over a hand that is still at work.

## Look over the mending before ye set sail {#review-fixes-before-committing}

The generated pre-commit hook automatically stages the fixes that apply, so ye skip the "fail, git add, and commit again" dance. To mend the canvas but stop the voyage for review:

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

When a fixer changes a file, hk fails the hook, hauling her back to port, and leaves the fixes for ye to review and stage. Then try the commit again. `stage = false` on its own turns off staging without making the hook fail.

For a single run, use `--no-stage` to turn off automatic staging.

<span id="hook-behavior"></span>

## Order the hands with care {#order-steps-deliberately}

All hands haul at once, up to the job limit, unless coordination makes some of them wait:

- `depends = "eslint"` waits for the named step.
- `exclusive = true` waits for earlier steps and holds back later ones until it's done.
- A `Group` draws a line across the deck: its children haul together, and later groups wait.
- Read/write locks, the lashings, coordinate steps whose files overlap.

Locks stop two hands writing the same file at once; they don't choose the final style when tools disagree. Configure rules that agree, or declare an explicit dependency.

## Letting a hand fail {#allowing-a-step-to-fail}

Set `allow_failure = true` on a step to run it and report a non-zero command
exit without failing the hook. 'Tis narrower than bypassing the hook or
skipping the step: every other step still blocks as it normally would, and
errors from hk itself are still fatal.

The setting can be an expression using `env(name)` when the rule should hang
on a standing order, an environment variable:

```pkl
["cargo-check"] {
    check = "cargo check"
    allow_failure = "env('KNOWN_BROKEN') == 'true'"
}
```

So `KNOWN_BROKEN=true git commit` will show a failed `cargo-check` but let
the commit sail, while an ordinary `git commit` is still blocked by the same
failure and hauled back to port.

## Commit-message hooks: the name ye christen her with {#commit-message-hooks}

`commit-msg` runs after the commit message is prepared and before the commit is made. Use the built-in Conventional Commits check, one of the standing crew:

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.5.0/hk@2.5.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.5.0/hk@2.5.0#/Builtins.pkl"

hooks {
  ["commit-msg"] {
    steps {
      ["conventional-commit"] = Builtins.check_conventional_commit
    }
  }
}
```

The `commit_msg_file` template variable holds the message's path. `prepare-commit-msg` also gets `source` and `sha` when Git supplies them. Use that hook to prepare or edit a message before the sailor's editor opens.

## When else Git sounds the pipe {#other-git-events}

hk answers these Git events with dedicated handlers:

| Event                | Template variables worth having aboard                    |
| -------------------- | --------------------------------------------------------- |
| `pre-commit`         | Staged file selection                                     |
| `pre-push`           | `hook_args` (remote and URL), `hook_stdin` (updated refs) |
| `commit-msg`         | `commit_msg_file`                                         |
| `prepare-commit-msg` | `commit_msg_file`, `source`, `sha`                        |
| `post-checkout`      | `prev_head`, `new_head`, `is_branch_checkout`             |
| `post-merge`         | `hook_args` (squash flag)                                 |
| `post-rewrite`       | `hook_args` (command), `hook_stdin` (rewritten refs)      |
| `pre-rebase`         | `hook_args` (upstream and optional branch)                |
| `post-commit`        | No arguments of its own                                   |

Dedicated handlers also hand over their raw arguments as `hook_args`. See the [run reference](/cli/run) for the details of each argument. Custom hooks can be called by name; hooks without a dedicated handler get an empty `hook_args` value.

These variables are also on hand in `condition` and `step_condition` expressions. For example, to run a `post-checkout` step only on branch checkouts, and report it as skipped on file checkouts:

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

## Let a hook or a hand sit this one out {#skip-a-hook-or-step}

```sh
HK_SKIP_STEPS=eslint git commit
HK_SKIP_HOOK=pre-push git push
HK=0 git commit
```

`HK=0` bypasses hk's installed hook launcher, so the pipe stays silent. To make a preference stick, use [Git configuration](/configuration#git-configuration), such as `git config --local hk.skipSteps eslint`.
