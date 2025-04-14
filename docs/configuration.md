---
outline: "deep"
---

# Configuration

## `hk.pkl`

hk is configured via `hk.pkl` which is written in [pkl-lang](https://pkl-lang.org/) from Apple.

Here's a basic `hk.pkl` file:

```pkl
amends "package://github.com/jdx/hk/releases/download/v0.7.5/hk@0.7.5#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v0.7.5/hk@0.7.5#/builtins.pkl"

local linters {
    // linters can be manually defined
    ["eslint"] {
        // the files to run the linter on, if no files are matched, the linter will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = List("*.js", "*.ts")
        // the command to run that makes no changes
        check = "eslint {{files}}"
        // the command to run that fixes the files (used by default)
        fix = "eslint --fix {{files}}"
    }
    // linters can also be specified with the builtins pkl library
    ["prettier"] = builtins.prettier
}

hooks {
    ["pre-commit"] {
        fix = true
        steps = linters
    }
    ["pre-push"] {
        steps = linters
    }
    // "fix" and "check" are special steps for `hk fix` and `hk check` commands
    ["fix"] {
        fix = true
        steps = linters
    }
    ["check"] {
        steps = linters
    }
}
```

The first line (`amends`) is critical because that imports the base configuration pkl for extending.

### `hooks.<HOOK>.steps`

Steps are the individual commands that make up a hook. They are executed in the order they are defined in parallel up to [`HK_JOBS`](/configuration#hk-jobs) at a time.

### `hooks.<HOOK>.steps.<STEP>.glob: List<String>`

Files the step should run on. By default this will only run this step if at least 1 staged file matches the glob patterns. If no patterns are provided, the step will always run.

### `hooks.<HOOK>.steps.<STEP>.check: String`

A command to run that does not modify files. This typically is a "check" command like `eslint` or `prettier --check` that returns a non-zero exit code if there are errors.
Parallelization works better with check commands than fix commands as no files are being modified.

```pkl
hooks {
    ["pre-commit"] {
        ["prettier"] {
            check = "prettier --check {{files}}"
        }
    }
}
```

Template variables:

- <code v-pre>{{files}}</code>: A list of files to run the linter on.

### `hooks.<HOOK>.steps.<STEP>.check_list_files: String`

A command that returns a list of files that need fixing. This is used to optimize the fix step when `check_first` is enabled. Instead of running the fix command on all files, it will only run on files that need fixing.

```pkl
hooks {
    ["pre-commit"] {
        ["prettier"] {
            check_list_files = "prettier --list-different {{files}}"
        }
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.check_diff: String`

A command that shows the diff of what would be changed. This is an alternative to `check` that can provide more detailed information about what would be changed.

### `hooks.<HOOK>.steps.<STEP>.fix: String`

A command to run that modifies files. This typically is a "fix" command like `eslint --fix` or `prettier --write`. Templates variables are the same as for `check`.

```pkl
local linters {
    ["prettier"] {
        fix = "prettier --write {{files}}"
    }
}
```

By default, hk will use `fix` commands but this can be overridden by setting [`HK_FIX=0`](/configuration#hk-fix) or running `hk run <HOOK> --run`.

### `hooks.<HOOK>.steps.<STEP>.check_first: bool`

Default: `true`

If true, hk will run the check step first and only run the fix step if the check step fails.

### `hooks.<HOOK>.steps.<STEP>.batch: bool`

Default: `false`

If true, hk will run the linter on batches of files instead of all files at once. This takes advantage of parallel processing for otherwise single-threaded linters like eslint and prettier.

```pkl
local linters {
    ["eslint"] {
        batch = true
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.stomp: bool`

Default: `false`

If true, hk will get a write lock instead of a read lock when running fix/fix_all. Use this if the tool has its own locking mechanism or you simply don't care if files may be written to by multiple linters simultaneously.

### `hooks.<HOOK>.steps.<STEP>.workspace_indicator: String`

If set, run the linter on workspaces only which are parent directories containing this filename. This is useful for tools that need to be run from a specific directory, like a project root.

```pkl
local linters {
    ["cargo-clippy"] {
        workspace_indicator = "Cargo.toml"
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.prefix: String`

If set, run the linter scripts with this prefix, e.g.: "mise exec --" or "npm run".

```pkl
local linters {
    ["eslint"] {
        prefix = "npm run"
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.dir: String`

If set, run the linter scripts in this directory.

```pkl
local linters {
    ["eslint"] (builtins.eslint) {
        dir = "frontend"
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.profiles: List<String>`

Profiles are a way to enable/disable linters based on the current profile. The linter will only run if its profile is in [`HK_PROFILE`](/configuration#hk-profile).

```pkl
local linters {
    ["prettier"] (builtins.prettier) {
        profiles = List("slow")
    }
}
```

Profiles can be prefixed with `!` to disable them.

```pkl
local linters {
    ["prettier"] (builtins.prettier) {
        profiles = List("!slow")
    }
}
```

### `hooks.<HOOK>.steps.<STEP>.depends: List<String>`

A list of steps that must finish before this step can run.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["prettier"] {
                depends = List("eslint")
            }
        }
    }
}
```

### `hooks.<HOOK>`

Hooks define when and how linters are run. The available hooks are:

- `pre-commit`
- `pre-push`
- TODO: add more

### `hooks.<HOOK>.steps`

Steps are the individual commands that make up a hook. They are executed in the order they are defined in parallel up to [`HK_JOBS`](/configuration#hk-jobs) at a time.

### `hooks.<HOOK>.steps.<STEP>.depends: List<String>`

A list of steps that must finish before this step can run.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["prettier"] {
                depends = List("eslint")
            }
        }
    }
}
```

### `hooks.<HOOK>.<STEP>.stage: List<String>`

A list of globs of files to add to the git index after running a fix step.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["prettier"] {
                stage = List("*.js", "*.ts")
            }
        }
    }
}
```

By default, all modified files will be added to the git index.

### `hooks.<HOOK>.steps.<STEP>.exclusive: bool`

Default: `false`

If true, this step will wait for any previous steps to finish before running. No other steps will start until this one finishes.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["prelint"] {
                exclusive = true // blocks other steps from starting until this one finishes
                run = "mise run prelint"
            }
            // ... other steps will run in parallel ...
            ["postlint"] {
                exclusive = true // wait for all previous steps to finish before starting
                run = "mise run postlint"
            }
        }
    }
}
```

### Alternative Syntax

You can also write `hk.json|hk.yaml|hk.toml` as an alternative to pkl, however builtins will not be available.
This may go away in the future so let me know if you rely on it.

## Environment Variables

Environment variables can be used to configure hk.

### `HK_CACHE_DIR`

Type: `path`
Default: `~/.cache/hk`

The cache directory to use.

### `HK_CHECK_FIRST`

Type: `bool`
Default: `true`

If true, hk will run check commands first then run fix commands if check fails iff there are multiple linters with the same file in a matching glob pattern.

The reason for this is to make hk able to parallelize as much as possible. We can have as many check commands running in parallel against
the same file as we want without them interfering with each other—however we can't have 2 fix commands potentially writing to the same file. So we optimistically run the check commands in parallel, then if any fail we run their fix commands potentially in series.

If this is disabled hk will have simpler logic that just uses fix commands in series in this situation.

### `HK_PROFILE`

Type: `string[]` (comma-separated list)

The profile(s) to use.

### `HK_FILE`

Type: `string`
Default: `hk.pkl` | `hk.toml` | `hk.yaml` | `hk.yml` | `hk.json`

The file to use for the configuration.

### `HK_FIX`

Type: `bool`
Default: `true`

If set to `false`, hk will not run fix steps.

### `HK_JOBS`

Type: `usize`
Default: `(number of cores)`

The number of jobs to run in parallel.

### `HK_LOG`

Type: `trace` | `debug` | `info` | `warn` | `error`
Default: `info`

The log level to use.

### `HK_LOG_FILE`

Type: `path`
Default: `~/.local/state/hk/hk.log`

The log file to use.

### `HK_LOG_FILE_LEVEL`

Type: `trace` | `debug` | `info` | `warn` | `error`
Default: `HK_LOG`

The log level to use for the log file.

### `HK_MISE`

Type: `bool`
Default: `false`

If set to `true`:

- When installing hooks with `hk install`, hk will use `mise x` to execute hooks which won't require activating mise to use mise tools
- When generating files with `hk generate`, hk will create a `mise.toml` file with hk configured

### `HK_SKIP_STEPS`

Type: `string[]` (comma-separated list)

A comma-separated list of step names to skip when running pre-commit and pre-push hooks.
For example: `HK_SKIP_STEPS=lint,test` would skip any steps named "lint" or "test".

### `HK_SKIP_HOOK`

Type: `string[]` (comma-separated list)
Default: `(empty)`

A comma-separated list of hook names to skip entirely. This allows you to disable specific git hooks from running.
For example: `HK_SKIP_HOOK=pre-commit,pre-push` would skip running those hooks completely.

This is useful when you want to temporarily disable certain hooks while still keeping them configured in your `hk.pkl` file.
Unlike `HK_SKIP_STEPS` which skips individual steps, this skips the entire hook and all its steps.

### `HK_STASH`

Type: `git` | `patch-file` | `none`
Default: `patch-file`

- `git`: Use `git stash` to stash unstaged changes before running hooks.
- `patch-file`: Use an hk generated patch file to stash unstaged changes before running hooks (typically faster and avoids `index is locked` errors).
- `none`: Do not stash unstaged changes before running hooks. Much faster but will stage unstaged changes if they are in the same file as staged changes with fix modifications.

### `HK_STASH_NO_GIT`

Type: `bool`
Default: `false`

If set to `true`, hk will not use `git stash` to stash unstaged changed and instead stash with internal diff logic. This gets around
`index is locked` errors when using `git stash` however it also means
that if hk crashes it will lose unstaged changes.

### `HK_STATE_DIR`

Type: `path`
Default: `~/.local/state/hk`

The state directory to use.

### `HK_HIDE_WHEN_DONE`

Type: `bool`
Default: `false`

If set to `true`, hk will hide the progress output when the hook finishes if there are no errors.
