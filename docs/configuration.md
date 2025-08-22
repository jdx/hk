---
outline: "deep"
---

# Configuration

## `hk.pkl`

hk is configured via `hk.pkl` which is written in [pkl-lang](https://pkl-lang.org/) from Apple.

Here's a basic `hk.pkl` file:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Builtins.pkl"

local linters = new Mapping<String, Step> {
    // linters can be manually defined
    ["eslint"] {
        // the files to run the linter on, if no files are matched, the linter will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = List("*.js", "*.ts")
        // these files will be staged after the fix step modifies them
        stage = List("*.js", "*.ts")
        // the command to run that makes no changes
        check = "eslint {{files}}"
        // the command to run that fixes the files (used by default)
        fix = "eslint --fix {{files}}"
    }
    // linters can also be specified with the Builtins pkl library
    ["prettier"] = Builtins.prettier
}

hooks {
    ["pre-commit"] {
        fix = true       // runs the fix step to make modifications
        stash = "git"    // stashes unstaged changes before running fix steps
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
        // optional: run a report after the hook finishes; HK_REPORT_JSON contains timing JSON
        report = #"node scripts/upload-timings.js <<<"$HK_REPORT_JSON""#
    }
}
```

The first line (`amends`) is critical because that imports the base configuration pkl for extending.

## `env: Mapping<String, String>`

Environment variables can be set in hk.pkl for configuring hk or the linters.

```pkl
env {
    ["HK_FAIL_FAST"] = "0"
    ["NODE_ENV"] = "production"
}
```

## `hooks.<HOOK>`

Hooks define when and how linters are run. See [hooks](/hooks) for more information.

## `hooks.<HOOK>.fix: bool`

Default: `false` (`true` for `pre-commit` and `fix`)

If true, hk will run the fix step to make modifications.

## `hooks.<HOOK>.stash: String`

Default: `git`

- `git`: Use `git stash` to stash unstaged changes before running fix steps.
- `patch-file`: Use an hk generated patch file to stash unstaged changes before running fix steps—typically faster.
- `none`: Do not stash unstaged changes before running fix steps.

## `hooks.<HOOK>.steps.<STEP|GROUP>`

Steps are the individual linters that make up a hook. They are executed in the order they are defined in parallel up to [`HK_JOBS`](/configuration#hk-jobs) at a time.

### `<STEP>.glob: List<String>`

Files the step should run on. By default this will only run this step if at least 1 staged file matches the glob patterns. If no patterns are provided, the step will always run.

### `<STEP>.check: (String | Script)`

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

If you want to use a different check command for different operating systems, you can define a Script instead of a String:

```pkl
hooks {
    ["pre-commit"] {
        ["prettier"] {
            check = new Script {
                linux = "prettier --check {{files}}"
                macos = "prettier --check {{files}}"
                windows = "prettier --check {{files}}"
                other = "prettier --check {{files}}"
            }
        }
    }
}
```

Template variables:

- `{{files}}`: A list of files to run the linter on.
- `{{workspace}}`: When `workspace_indicator` is set and matched, this is the workspace directory path (e.g., `.` for the repo root or `packages/app`).
- `{{workspace_indicator}}`: Full path to the matched workspace indicator file (e.g., `packages/app/package.json`).
- `{{workspace_files}}`: A list of files relative to `{{workspace}}`.

### `<STEP>.check_list_files: (String | Script)`

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

### `<STEP>.check_diff: (String | Script)`

A command that shows the diff of what would be changed. This is an alternative to `check` that can provide more detailed information about what would be changed.

### `<STEP>.fix: (String | Script)`

A command to run that modifies files. This typically is a "fix" command like `eslint --fix` or `prettier --write`. Templates variables are the same as for `check`.

```pkl
local linters = new Mapping<String, Step> {
    ["prettier"] {
        fix = "prettier --write {{files}}"
    }
}
```

By default, hk will use `fix` commands but this can be overridden by setting [`HK_FIX=0`](/configuration#hk-fix) or running `hk run <HOOK> --run`.

### `<STEP>.check_first: bool`

Default: `true`

If true, hk will run the check step first and only run the fix step if the check step fails.

### `<STEP>.batch: bool`

Default: `false`

If true, hk will run the linter on batches of files instead of all files at once. This takes advantage of parallel processing for otherwise single-threaded linters like eslint and prettier.

```pkl
local linters = new Mapping<String, Step> {
    ["eslint"] {
        batch = true
    }
}
```

### `<STEP>.stomp: bool`

Default: `false`

If true, hk will get a write lock instead of a read lock when running fix/fix_all. Use this if the tool has its own locking mechanism or you simply don't care if files may be written to by multiple linters simultaneously.

### `<STEP>.workspace_indicator: String`

If set, run the linter on workspaces only which are parent directories containing this filename. This is useful for tools that need to be run from a specific directory, like a project root.

```pkl
local linters = new Mapping<String, Step> {
    ["cargo-clippy"] {
        workspace_indicator = "Cargo.toml"
            glob = "*.rs"
            workspace_indicator = "Cargo.toml"
            check = "cargo clippy --manifest-path {{workspace_indicator}}"
    }
}
```

In this example, given a file list like the following:

```text
└── workspaces/
    ├── proj1/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       └── main.rs
    └── proj2/
        ├── Cargo.toml
        └── src/
            ├── lib.rs
            └── main.rs
```

hk will run 1 step for each workspace even though multiple rs files are in each workspace:

- `cargo clippy --manifest-path workspaces/proj1/Cargo.toml`
- `cargo clippy --manifest-path workspaces/proj2/Cargo.toml`

When `workspace_indicator` is used, the following template variables become available in commands and env:

- `{{workspace}}`: the workspace directory path
- `{{workspace_indicator}}`: the matched indicator file path
- `{{workspace_files}}`: files relative to `{{workspace}}`

For example, in a monorepo with Node packages:

```pkl
local linters = new Mapping<String, Step> {
  ["npm-lint"] {
    glob = List("*.js", "*.jsx", "*.ts", "*.tsx")
    workspace_indicator = "package.json"
    check = "echo cd {{workspace}} && npm run lint -- {{workspace_files}}"
    fix = "echo cd {{workspace}} && npm run fix -- {{workspace_files}}"
  }
}
```

### `<STEP>.prefix: String`

If set, run the linter scripts with this prefix, e.g.: "mise exec --" or "npm run".

```pkl
local linters = new Mapping<String, Step> {
    ["eslint"] {
        prefix = "npm run"
    }
}
```

### `<STEP>.dir: String`

If set, run the linter scripts in this directory.

```pkl
local linters = new Mapping<String, Step> {
    ["eslint"] = (Builtins.eslint) {
        dir = "frontend"
    }
}
```

### `<STEP>.profiles: List<String>`

Profiles are a way to enable/disable linters based on the current profile. The linter will only run if its profile is in [`HK_PROFILE`](/configuration#hk-profile).

```pkl
local linters = new Mapping<String, Step> {
    ["prettier"] = (Builtins.prettier) {
        profiles = List("slow")
    }
}
```

Profiles can be prefixed with `!` to disable them.

```pkl
local linters = new Mapping<String, Step> {
    ["prettier"] = (Builtins.prettier) {
        profiles = List("!slow")
    }
}
```

### `<STEP>.depends: List<String>`

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

### `<STEP>.shell: (String | Script)`

If set, use this shell instead of the default `sh -o errexit -c`.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["prettier"] {
                shell = "bash -o errexit -c"
            }
        }
    }
}

### `<STEP>.stage: List<String>`

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

### `<STEP>.exclusive: bool`

Default: `false`

If true, this step will wait for any previous steps to finish before running. No other steps will start until this one finishes. Under
the hood this groups the previous steps into a group.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["prelint"] {
                exclusive = true // blocks other steps from starting until this one finishes
                check = "mise run prelint"
            }
            // ... other steps will run in parallel ...
            ["postlint"] {
                exclusive = true // wait for all previous steps to finish before starting
                check = "mise run postlint"
            }
        }
    }
}
```

### `<STEP>.exclude: (String | List<String>)`

A list of glob patterns to exclude from the step. Files matching these patterns will be skipped.

```pkl
local linters = new Mapping<String, Step> {
    ["prettier"] {
        exclude = List("*.js", "*.ts")
    }
}
```

### `<STEP>.interactive: bool`

Default: `false`

If true, connects stdin/stdout/stderr to hk's execution. This implies `exclusive = true`.

```pkl
local linters = new Mapping<String, Step> {
    ["show-warning"] {
        interactive = true
        check = "echo warning && read -p 'Press Enter to continue'"
    }
}
```

### `<STEP>.condition: String`

If set, the step will only run if this condition evaluates to true. Evaluated with [`expr`](https://github.com/jdx/expr-rs).

```pkl
local linters = new Mapping<String, Step> {
    ["prettier"] {
        condition = "eval('test -f check.js')"
    }
}
```

### `<STEP>.hide: bool`

Default: `false`

If true, the step will be hidden from output.

```pkl
local linters = new Mapping<String, Step> {
    ["prettier"] {
        hide = true
    }
}
```

### `<STEP>.output_summary: "stdout" | "stderr" | "combined" | "hide"`

Default: `"stderr"`

Controls which stream(s) from the step’s command are captured and printed at the end of the hook run. This prints a single consolidated block per step that produced any output, with a header like `STEP_NAME stderr:`.

- `"stderr"` (default): capture only standard error
- `"stdout"`: capture only standard output
- `"combined"`: capture both stdout and stderr interleaved (line-by-line as produced)
- `"hide"`: capture nothing and print nothing for this step

Examples:

```pkl
hooks {
  ["check"] {
    steps {
      ["lint"] {
        check = "eslint {{files}}"
        output_summary = "combined"
      }
      ["format"] {
        check = "prettier --check {{files}}"
        output_summary = "stdout"
      }
      ["quiet-step"] {
        check = "echo noisy && echo warn 1>&2"
        output_summary = "hide"
      }
    }
  }
}
```

#### Git status in conditions and templates

hk provides the current git status to both condition expressions and Tera templates via a `git` object. This lets you avoid shelling out in conditions (e.g., `exec('git …')`).

- Available fields: `git.staged_files`, `git.unstaged_files`, `git.untracked_files`, `git.modified_files`
- In conditions (expr):

```pkl
// Run only if there are any staged files
condition = "git.staged_files != []"

// Run only if a maintainers file is staged
condition = "git.staged_files | any(f, f.ends_with(\"maintainers.yml\"))"
```

- In templates (Tera):

```pkl
check = "echo staged: {{ git.staged_files }}"
```

These lists contain repository-relative paths for files currently in each state.

### `<STEP>.env: Mapping<String, String>`

Environment variables specific to this step. These are merged with the global environment variables.

```pkl
local linters = new Mapping<String, Step> {
    ["prettier"] {
        env {
            ["NODE_ENV"] = "production"
        }
    }
}
```

### `<STEP>.tests: Mapping<String, StepTest>`

Define self-contained tests for a step, runnable via `hk test`.

Key points:

- Mapping is keyed by test name.
- Supported run modes: `check` or `fix` (defaults to `check`).
- `files` is optional; if omitted, it defaults to the keys of `write`.
- `write` lets you create files before the test runs (paths can be relative to the sandbox or absolute).
- `fixture` copies a directory into a temporary sandbox before the test runs.
- `env` merges with the step’s `env` (test env wins on conflicts).
- `expect` supports:
  - `code` (default 0)
  - `stdout`, `stderr` substring checks
  - `files` full-file content assertions

Template variables available in tests are the same as for steps, plus:

- `{{files}}`, `{{globs}}`, `{{workspace}}`, `{{workspace_indicator}}`
- `{{root}}`: project root
- `{{tmp}}`: sandbox path used to execute the test

Example:

```pkl
hooks {
  ["check"] {
    steps {
      ["prettier"] {
        check = "prettier --check {{ files }}"
        fix = "prettier --write {{ files }}"
        tests {
          ["formats json via fix"] {
            run = "fix"
            write { ["{{tmp}}/a.json"] = "{\"b\":1}" }
            // files omitted -> defaults to write keys
            expect { files { ["{{tmp}}/a.json"] = "{\n  \"b\": 1\n}\n" } }
          }
          ["check shows output"] {
            run = "check"
            files = List("{{tmp}}/a.json")
            env { ["FOO"] = "bar" }
            expect { stdout = "prettier" }
          }
        }
      }
    }
  }
}
```

Run tests with:

```bash
hk test                 # all tests
hk test --step prettier # only prettier’s tests
hk test --name formats json via fix
hk test --list          # list without running
```

### `<GROUP>`

A group is a collection of steps that are executed in parallel, waiting for previous steps/groups to finish and blocking other steps/groups from starting until it finishes. This is a naive way to ensure the order of execution. It's better to make use of read/write locks and depends.

```pkl
hooks {
    ["pre-commit"] {
        steps {
            ["build"] = new Group {
                steps = new Mapping<String, Step> {
                    ["ts"] = new Step {
                        fix = "tsc -b"
                    }
                    ["rs"] = new Step {
                        fix = "cargo build"
                    }
                }
            }
            // these steps will run in parallel after the build group finishes
            ["lint"] = new Group {
                steps = new Mapping<String, Step> {
                    ["prettier"] = new Step {
                        check = "prettier --check {{files}}"
                    }
                    ["eslint"] = new Step {
                        check = "eslint {{files}}"
                    }
                }
            }
        }
    }
}
```

## `hkrc`

The `hkrc` is a global configuration file that allows you to customize hk's behavior across all projects. By default, hk will look for this file in your home directory. You can override its location using the `--hkrc` flag.

The hkrc file follows the same format as `hk.pkl` and can be used to define global hooks and linters that will be applied to all projects. This is useful for setting up consistent linting rules across multiple repositories.

Example hkrc file:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Builtins.pkl"

local linters {
    ["prettier"] = Builtins.prettier
    ["eslint"] {
        glob = List("*.js", "*.ts")
        check = "eslint {{files}}"
        fix = "eslint --fix {{files}}"
    }
}

hooks {
    ["pre-commit"] {
        fix = true
        steps = linters
    }
}
```

The hkrc configuration is applied after loading the project configuration (`hk.pkl`), which means:

- User configuration takes precedence over project configuration
- Project-specific settings in `hk.pkl` can override or extend the global configuration
