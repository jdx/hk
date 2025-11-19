---
outline: "deep"
---

# Configuration

## `hk.pkl`

hk is configured via `hk.pkl` which is written in [pkl-lang](https://pkl-lang.org/) from Apple.

Here's a basic `hk.pkl` file:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Builtins.pkl"

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

## `default_branch: String`

Default: auto-detected

Specifies the preferred default branch to compare against when hk needs a reference (e.g., suggestions in pre-commit warnings). If unset or empty, hk attempts to detect it via `origin/HEAD`, the current branch's remote, or falls back to `main`/`master` if they exist on the remote.

Examples:

```pkl
// Use a local branch name
default_branch = "main"

// Or a remote-qualified ref
default_branch = "origin/main"
```

Notes:

- Both local branch names (e.g., `main`) and remote-qualified refs (e.g., `origin/main`) are supported.
- If omitted, hk will detect the default branch based on your repository's remotes and branches.

## `env: Mapping<String, String>`

Environment variables can be set in hk.pkl for configuring the linters.

```pkl
env {
    ["NODE_ENV"] = "production"
}
```

## `exclude: (String | List<String> | Regex)`

Default: `(empty)`

Global exclude patterns that apply to all hooks and steps. Files matching these patterns will be skipped from processing. Supports directory names, glob patterns, and regex patterns.

```pkl
// Exclude specific directories
exclude = List("node_modules", "dist", "build")

// Exclude using glob patterns
exclude = List("**/*.min.js", "**/*.map", "**/vendor/**")

// Single pattern
exclude = "node_modules"

// Exclude using regex pattern (for complex matching)
// First import Types.pkl to use the Regex helper
import "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Types.pkl"
exclude = Types.Regex(#".*\.(test|spec)\.(js|ts)$"#)
```

Notes:

- Patterns from all configuration sources are unioned together
- Simple directory names automatically match their contents (e.g., `"excluded"` matches `excluded/*` and `excluded/**`)
- Can be overridden per-step with `<STEP>.exclude`
- Regex patterns use Rust regex syntax and match against full file paths

## `fail_fast: Boolean`

Default: `true`

Controls whether hk aborts remaining steps/groups after the first failure.

- When `true`, as soon as a step fails, hk cancels pending steps in the same hook and returns the error.
- When `false`, hk continues running other steps and reports all failures at the end.

## `hooks.<HOOK>`

Hooks define when and how linters are run. See [hooks](/hooks) for more information.

## `hooks.<HOOK>.fix: bool`

Default: `false` (`true` for `fix`)

If true, hk will run the fix command for each step (if it exists) to make modifications.

## `hooks.<HOOK>.stash: (String | Boolean)`

Default: `"none"`

- `"git"`: Use `git stash` to stash unstaged changes before running fix steps.
- `"patch-file"`: Alias of `git` behavior for now.
- `"none"`: Do not stash unstaged changes before running fix steps.
- `true` (boolean): Alias of `"git"`.
- `false` (boolean): Alias of `"none"`.

Examples:

```pkl
hooks {
  ["pre-commit"] {
    fix = true
    stash = true        // boolean shorthand for git
    steps = linters
  }

  ["fix"] {
    fix = true
    stash = "none"      // disable stashing
    steps = linters
  }
}
```

## `hooks.<HOOK>.steps.<STEP|GROUP>`

Steps are the individual linters that make up a hook. They are executed in the order they are defined in parallel up to [`HK_JOBS`](/configuration#hk-jobs) at a time.

### `<STEP>.glob: (String | List<String> | Regex)`

Files the step should run on. By default this will only run this step if at least 1 staged file matches the glob or regex patterns. If no patterns are provided, the step will always run.

```pkl
// Glob patterns
["prettier"] {
    glob = List("*.js", "*.ts")
    check = "prettier --check {{files}}"
}

// Single glob pattern
["eslint"] {
    glob = "*.js"
    check = "eslint {{files}}"
}

// Regex pattern for complex matching
["config-lint"] {
    glob = Types.Regex(#"^(config|settings).*\.(json|yaml|yml)$"#)
    check = "config-lint {{files}}"
}
```

### `<STEP>.types: List<String>`

Default: `(none)`

Filter files by their type rather than just glob patterns. Matches files by extension, shebang, or content detection (OR logic - file must match ANY of the specified types). This is particularly useful for matching scripts without file extensions.

```pkl
// Match Python files by extension AND shebang (including extensionless scripts)
["black"] {
    types = List("python")
    fix = "black {{files}}"
}

// Match shell scripts by extension or shebang
["shellcheck"] {
    types = List("shell")
    check = "shellcheck {{files}}"
}

// Match multiple types (OR logic)
["format-scripts"] {
    types = List("python", "shell", "ruby")
    fix = "format-script {{files}}"
}

// Combine types with glob patterns for more precise filtering
["format-src-python"] {
    glob = "src/**/*"  // Only files in src/
    types = List("python")  // That are Python files
    fix = "black {{files}}"
}
```

**Supported types include:**

- **Languages:** `python`, `javascript`, `typescript`, `ruby`, `go`, `rust`, `java`, `kotlin`, `swift`, `c`, `c++`, `csharp`, `php`
- **Shells:** `shell`, `bash`, `zsh`, `fish`, `sh`
- **Data formats:** `json`, `yaml`, `toml`, `xml`, `csv`
- **Markup:** `html`, `markdown`, `css`
- **Special:** `text`, `binary`, `executable`, `symlink`, `dockerfile`
- **Images:** `image`, `png`, `jpeg`, `gif`, `svg`, `webp`
- **Archives:** `archive`, `zip`, `tar`, `gzip`

Types are detected using:
1. File extension (e.g., `.py` → `python`)
2. Shebang line (e.g., `#!/usr/bin/env python3` → `python`)
3. Special filenames (e.g., `Dockerfile` → `dockerfile`)
4. Content/magic number detection for binary files

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
```

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

### `<STEP>.exclude: (String | List<String> | Regex)`

Files to exclude from the step. Supports glob patterns and regex patterns. Files matching these patterns will be skipped.

```pkl
// Exclude with glob patterns
["prettier"] {
    glob = List("**/*.yaml")
    exclude = List("*.test.yaml", "*.fixture.yaml")
    check = "prettier --check {{files}}"
}

// Exclude with regex pattern for complex matching
["linter"] {
    glob = List("**/*")
    exclude = Types.Regex(#"""
(?x)
^(vendor|dist|build)/.*$|
.*\.(min|bundle)\.(js|css)$|
.*\.generated\.(ts|js)$
"""#)
    check = "custom-lint {{files}}"
}
```

Notes:
- Regex patterns use Rust regex syntax
- The `(?x)` flag enables verbose mode for multi-line patterns with comments
- Use raw strings (`#"..."#` or `#"""..."""#`) to avoid escaping backslashes

**Using the Regex helper**

The `Regex()` helper function is available by importing Types.pkl:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Types.pkl"

// Use it like:
exclude = Types.Regex(#".*\.test\.js$"#)
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
  - Staged classifications: `git.staged_added_files`, `git.staged_modified_files`, `git.staged_deleted_files`, `git.staged_renamed_files`, `git.staged_copied_files`
  - Unstaged classifications: `git.unstaged_modified_files`, `git.unstaged_deleted_files`, `git.unstaged_renamed_files`

- In conditions (expr):

```pkl
// Run only if there are any staged files
condition = "git.staged_files != []"

// Run only if a Cargo.toml file is staged
condition = #"any(git.staged_files, {hasSuffix(#, "Cargo.toml")})"#

// Diff-filter approximations
// Added or Renamed (AR):
condition = "(git.staged_added_files != []) || (git.staged_renamed_files != [])"

// Renamed or Deleted (RD):
condition = "(git.staged_renamed_files != []) || (git.staged_deleted_files != [])"
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
- `before` is an optional shell command to run before the test's main command. If it fails (non-zero exit), the test fails immediately.
- `after` is an optional shell command to run after the main command, before evaluating expectations. If it fails, the test fails and reports that failure.
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
          ["before generates file, after verifies contents"] {
            run = "fix"
            // before: generate an input file the step will process
            before = #"printf '{\"b\":1}' > {{tmp}}/raw.json"#
            // files: tell hk which file the step should operate on
            files = List("{{tmp}}/raw.json")
            // after: verify the contents using a shell assertion
            after = #"grep -q '\"b\": 1' {{tmp}}/raw.json"#
            // expect: full-file match after formatting
            expect { files { ["{{tmp}}/raw.json"] = "{\n  \"b\": 1\n}\n" } }
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
amends "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Builtins.pkl"

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

## Settings Reference

This section lists the configuration settings that control how hk behaves. Settings are sourced from multiple places; higher precedence overrides lower. Some list settings (e.g., `exclude`, `skip_steps`, `skip_hooks`, `hide_warnings`) use union semantics, combining values from multiple sources.

| Precedence | Source | Example |
|---|---|---|
| 1 | CLI flags | `hk check --fail-fast` |
| 2 | Environment variables (HK_*) | `HK_JOBS=8 hk check` |
| 3 | Git config (local repo) | `git config --local hk.jobs 4` |
| 4 | Git config (global/system) | `git config --global hk.failFast false` |
| 5 | User rc (.hkrc.pkl) | `jobs = 4` in `~/.hkrc.pkl` |
| 6 | Project config (hk.pkl) | `jobs = 4` in `hk.pkl` |
| 7 | Built-in defaults | `jobs = 0` (auto, CPU cores) |

### Git Configuration

hk can be configured through git config. All git config keys use the `hk.` prefix:

```bash
# Set number of parallel jobs
git config --local hk.jobs 5

# Disable fail-fast behavior
git config --local hk.failFast false

# Add profiles
git config --local hk.profile slow
git config --local --add hk.profile fast

# Add exclude patterns (union semantics)
git config --local hk.exclude "node_modules"
git config --local --add hk.exclude "**/*.min.js"

# Skip specific steps
git config --local hk.skipSteps "slow-test,flaky-test"

# Skip entire hooks
git config --local hk.skipHook "pre-push"

# Configure warnings
git config --local hk.warnings "missing-profiles"
git config --local hk.hideWarnings "missing-profiles"
```

Git config supports both multivar entries (multiple values with the same key) and comma-separated values in a single entry.

### User Configuration (`.hkrc.pkl`)

User-specific defaults can be set in `~/.hkrc.pkl`:

```pkl
amends "package://github.com/jdx/hk/releases/latest/hk#/UserConfig.pkl"

jobs = 4
fail_fast = false
exclude = List("node_modules", "dist", "build")
skip_steps = List("slow-test")
skip_hooks = List("pre-push")
```

### Configuration Introspection

Use the `hk config` commands to inspect your configuration:

```bash
# Show effective configuration (all sources merged)
hk config dump

# Get a specific configuration value
hk config get exclude
hk config get skip_steps

# Show configuration source precedence
hk config sources
```

<!-- BEGIN: AUTO-GENERATED SETTINGS -->


### `all`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--all`

Enables running all available steps, including those that might be disabled by default or require specific profiles.

When enabled, hk will run all configured steps regardless of their profile requirements or other filtering criteria.
This is useful for comprehensive checking or when you want to run every possible linting/formatting task.

Example: `hk check --all` to run all available checks.

### `cache-dir`

- Type: `path`
- Sources:
  - ENV: `HK_CACHE_DIR`

Directory where hk stores cache files for improved performance.

Default location: `~/.cache/hk`

Cache includes tool binary locations, parsed configurations, and other performance optimizations.

### `check`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--check`, `-c`
  - ENV: `HK_CHECK`
  - Git: `hk.check`

Forces hk to run only check commands (read-only) instead of fix commands.

This is the opposite of the `fix` setting. When enabled, hk will report issues without attempting to fix them.

Useful for CI environments where you want to verify code quality without making changes.

### `check-first`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_CHECK_FIRST`
  - Git: `hk.checkFirst`

If enabled, hk will run check commands first, then run fix commands only if the check fails when there are multiple linters with the same file in matching glob patterns.

The reason for this optimization is to maximize parallelization. We can have multiple check commands running in parallel against the same file without interference, but we can't have 2 fix commands potentially writing to the same file simultaneously.

If disabled, hk will use simpler logic that just runs fix commands in series in this situation.

### `display-skip-reasons`

- Type: `list<string>`
- Default: `["profile-not-enabled"]`
- Sources:
  - ENV: `HK_DISPLAY_SKIP_REASONS`
  - Git: `hk.displaySkipReasons`
  - Pkl: `display_skip_reasons`

Controls which skip reasons are displayed when steps are skipped.

Available options:
- `all`: Show all skip reasons
- `none`: Hide all skip reasons
- `disabled-by-config`: Show when steps are skipped due to configuration
- `profile-not-enabled`: Show when steps are skipped due to missing profiles (default)

Example: `HK_DISPLAY_SKIP_REASONS=all` to see all skip reasons.

### `exclude`

- Type: `list<string>`
- Sources:
  - CLI: `--exclude`, `-e`
  - ENV: `HK_EXCLUDE`
  - Git: `hk.exclude`
  - Pkl: `exclude`

Glob patterns to exclude from processing. These patterns are **unioned** with exclude patterns from other configuration sources (git config, user config, project config). Supports both directory names and glob patterns.

Examples:
- Exclude specific directories: `node_modules,dist`
- Exclude using glob patterns: `**/*.min.js,**/*.map`

All exclude patterns from different sources are combined.

### `fail-fast`

- Type: `bool`
- Default: `true`
- Sources:
  - CLI: `--fail-fast`, `--no-fail-fast`
  - ENV: `HK_FAIL_FAST`
  - Git: `hk.failFast`
  - Pkl: `fail_fast`

Controls whether hk aborts running steps after the first one fails.

When enabled (default), hk will stop execution immediately when a step fails, providing quicker feedback.
When disabled, hk will continue running all steps even if some fail, useful for seeing all issues at once.

Can be toggled with `--fail-fast` / `--no-fail-fast` CLI flags.

### `fix`

- Type: `bool`
- Default: `true`
- Sources:
  - CLI: `--fix`, `-f`
  - ENV: `HK_FIX`
  - Git: `hk.fix`

Controls whether hk runs fix commands (that modify files) or check commands (read-only).

When enabled (default), runs fix commands to automatically resolve issues.
When disabled, only runs check commands to report issues without making changes.

Can be toggled with `--fix` / `--check` CLI flags.

### `hide-warnings`

- Type: `list<string>`
- Sources:
  - ENV: `HK_HIDE_WARNINGS`
  - Git: `hk.hideWarnings`
  - Pkl: `hide_warnings`

Warning tags to suppress. Allows hiding specific warning messages that you don't want to see.

Available warning tags:
- `missing-profiles`: Suppresses warnings about steps being skipped due to missing profiles

Example: `HK_HIDE_WARNINGS=missing-profiles`

All hide configurations from different sources are **unioned** together.

### `hide-when-done`

- Type: `bool`
- Default: `false`
- Sources:
  - ENV: `HK_HIDE_WHEN_DONE`

Controls whether hk hides the progress output when the hook finishes successfully.

When enabled, successful runs will clear their output to reduce visual clutter.
Failed runs will always show their output regardless of this setting.

### `hkrc`

- Type: `path`
- Default: `".hkrc.pkl"`
- Sources:
  - CLI: `--hkrc`

Path to the user configuration file.

Default: `.hkrc.pkl` in the current directory or parent directories.

This file can override project-level settings and is useful for personal preferences.

### `jobs`

- Type: `usize`
- Default: `0`
- Sources:
  - CLI: `--jobs`, `-j`
  - ENV: `HK_JOBS`, `HK_JOB`
  - Git: `hk.jobs`
  - Pkl: `jobs`

The number of parallel processes that hk will use to execute steps concurrently. This affects performance by controlling how many linting/formatting tasks can run simultaneously.

Set to `0` (default) to auto-detect based on CPU cores.

Example usage:
- `hk check --jobs 4` - Run with 4 parallel jobs
- `HK_JOBS=8 hk fix` - Set via environment variable

### `json`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--json`
  - ENV: `HK_JSON`
  - Git: `hk.json`

Enables JSON output format for structured data.

When enabled, hk outputs machine-readable JSON instead of human-readable text.
Useful for integration with other tools or for programmatic processing of results.

Example: `hk check --json | jq '.steps[] | select(.failed)'`

### `libgit2`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_LIBGIT2`

Controls whether hk uses libgit2 (a Git library) or shells out to git CLI commands.

When enabled (default), uses libgit2 for better performance in most cases.
When disabled, uses git CLI commands which may provide better performance in some cases such as when using `fsmonitor` to watch for changes.

### `log-file`

- Type: `path`
- Sources:
  - ENV: `HK_LOG_FILE`

Path to the log file where hk writes detailed execution logs.

Default location: `~/.local/state/hk/hk.log`

Useful for debugging issues or keeping an audit trail of hook executions.

### `log-file-level`

- Type: `enum`
- Default: `"info"`
- Sources:
  - ENV: `HK_LOG_FILE_LEVEL`

Controls the verbosity of file logging output.

Uses the same levels as `log_level` but specifically for the log file.
Defaults to the same level as `log_level` if not specified.

This allows you to have different verbosity levels for console and file output.

### `log-level`

- Type: `enum`
- Default: `"info"`
- Sources:
  - ENV: `HK_LOG`, `HK_LOG_LEVEL`

Controls the verbosity of console output.

Available levels (from least to most verbose):
- `off`: No logging
- `error`: Only errors
- `warn`: Errors and warnings
- `info`: Normal output (default)
- `debug`: Detailed debugging information
- `trace`: Very detailed trace information

Example: `HK_LOG_LEVEL=debug hk check`

### `mise`

- Type: `bool`
- Default: `false`
- Sources:
  - ENV: `HK_MISE`

Enables deep integration with [mise](https://mise.jdx.dev) for tool management.

When enabled:
- `hk install` will use `mise x` to execute hooks, ensuring mise tools are available without activation
- `hk init` will create a `mise.toml` file with hk configured
- Tool discovery will use mise shims automatically

### `no-progress`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--no-progress`

Disables progress bars and real-time status updates.

When enabled, hk will use simpler text output instead of dynamic progress indicators.
Useful for CI environments or when output is being logged to a file.

### `profiles`

- Type: `list<string>`
- Sources:
  - CLI: `--profile`, `-p`
  - ENV: `HK_PROFILE`, `HK_PROFILES`
  - Git: `hk.profile`
  - Pkl: `profiles`

Profiles to enable or disable. Profiles allow you to group steps that should run only in certain contexts (e.g., CI, slow tests).

Prefix with `!` to explicitly disable a profile.

Example usage:
- `HK_PROFILE=ci` - Enable the CI profile
- `HK_PROFILE=slow,ci` - Enable multiple profiles
- `--profile=!slow` - Explicitly disable the slow profile

### `quiet`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--quiet`, `-q`

Suppresses non-essential output.

When enabled, only errors and critical information will be displayed.
Useful for scripting or when you only care about the exit code.

### `silent`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--silent`

Completely suppresses all output, including errors.

More extreme than `quiet` - absolutely no output will be displayed.
Useful when only the exit code matters.

### `skip-hooks`

- Type: `list<string>`
- Sources:
  - ENV: `HK_SKIP_HOOK`, `HK_SKIP_HOOKS`
  - Git: `hk.skipHooks`, `hk.skipHook`
  - Pkl: `skip_hooks`

A list of hook names to skip entirely. This allows you to disable specific git hooks from running.

For example: `HK_SKIP_HOOK=pre-commit,pre-push` would skip running those hooks completely.

This is useful when you want to temporarily disable certain hooks while still keeping them configured in your `hk.pkl` file.
Unlike `skip_steps` which skips individual steps, this skips the entire hook and all its steps.

This setting can also be configured via:
- Git config: `git config hk.skipHook "pre-commit"`
- User config (`.hkrc.pkl`): `skip_hooks = List("pre-commit")`

**All skip configurations from different sources are unioned together.**

### `skip-steps`

- Type: `list<string>`
- Sources:
  - CLI: `--skip-step`
  - ENV: `HK_SKIP_STEPS`, `HK_SKIP_STEP`
  - Git: `hk.skipSteps`, `hk.skipStep`
  - Pkl: `skip_steps`

A list of step names to skip when running hooks. This allows you to bypass specific linting or formatting tasks.

For example: `HK_SKIP_STEPS=lint,test` would skip any steps named "lint" or "test".

This setting can also be configured via:
- Git config: `git config hk.skipSteps "step1,step2"`
- User config (`.hkrc.pkl`): `skip_steps = List("step1", "step2")`

**All skip configurations from different sources are unioned together.**

### `slow`

- Type: `bool`
- Default: `false`
- Sources:
  - CLI: `--slow`, `-s`

Enables the "slow" profile for running additional checks that may take longer.

This is a convenience flag equivalent to `--profile=slow`.

Useful for thorough checking in CI or before major releases.

### `stage`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_STAGE`
  - Git: `hk.stage`

Controls whether hk automatically stages files that were fixed by pre-commit hooks.

When enabled (default), files modified by fix commands will be automatically staged.
When disabled (`HK_STAGE=0`), fixed files will remain as unstaged changes, allowing you to review them before committing.

This is useful when you want to manually review changes made by auto-fixers before including them in your commit.

Example: `HK_STAGE=0 git commit -m "test"` to prevent auto-staging of generated files.

### `stash`

- Type: `enum`
- Default: `"auto"`
- Sources:
  - CLI: `--stash`
  - ENV: `HK_STASH`
  - Git: `hk.stash`

Strategy for temporarily saving uncommitted changes before running hooks that might modify files. This prevents conflicts between your working directory changes and automated fixes.

Available strategies:
- `auto`: Automatically choose the best strategy (default)
- `git`: Use `git stash` to stash changes
- `patch-file`: Use hk-generated patch files (typically faster, avoids "index is locked" errors)
- `none`: No stashing (fastest, but may cause staging conflicts if fixes modify unstaged changes in the same file)

### `stash-backup-count`

- Type: `usize`
- Default: `20`
- Sources:
  - ENV: `HK_STASH_BACKUP_COUNT`
  - Git: `hk.stashBackupCount`
  - Pkl: `stash_backup_count`

Number of backup patch files to keep per repository when using git stash.

Each time git stash is used, hk creates a backup patch file in
$HK_STATE_DIR/patches/. This setting controls how many of these
backups are retained per repository (oldest are automatically deleted).

Set to 0 to disable patch backup creation entirely.

Default: 20

### `stash-untracked`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_STASH_UNTRACKED`
  - Git: `hk.stashUntracked`

Controls whether untracked files are included when stashing before running hooks.

When enabled (default), untracked files will be temporarily stashed along with tracked changes.
This ensures a clean working directory for hook execution.

### `state-dir`

- Type: `path`
- Sources:
  - ENV: `HK_STATE_DIR`

Directory where hk stores persistent state files.

Default location: `~/.local/state/hk`

Includes logs, temporary patch files for stashing, and other state information.

### `summary-text`

- Type: `bool`
- Default: `false`
- Sources:
  - ENV: `HK_SUMMARY_TEXT`

Controls whether per-step output summaries are printed in plain text mode.

By default, summaries are only shown when hk is rendering progress bars (non-text mode).
Set to `true` to force summaries to appear in text mode, useful for CI environments.

Example: `HK_SUMMARY_TEXT=1 hk check`

### `terminal-progress`

- Type: `bool`
- Default: `true`
- Sources:
  - ENV: `HK_TERMINAL_PROGRESS`
  - Git: `hk.terminalProgress`
  - Pkl: `terminal_progress`

Enables or disables reporting progress via OSC sequences to compatible terminals.

### `timing-json`

- Type: `path`
- Sources:
  - ENV: `HK_TIMING_JSON`

Path to write a JSON timing report after a hook finishes. The report includes total wall time and per-step wall time, with overlapping intervals merged so time isn't double-counted across parallel step parts.

The `steps` field maps step names to objects containing:
- `wall_time_ms`: merged wall time in milliseconds
- `profiles` (optional): list of profiles required for that step

Example usage: `HK_TIMING_JSON=/tmp/hk-timing.json hk check`

Example output:
```json
{
  "total": { "wall_time_ms": 12456 },
  "steps": {
    "lint": { "wall_time_ms": 4321, "profiles": ["ci", "fast"] },
    "fmt": { "wall_time_ms": 2100 }
  }
}
```

When a hook-level `report` command is configured in `hk.pkl`, hk will set `HK_REPORT_JSON` to the same timing JSON content and execute the command after the hook finishes.

### `trace`

- Type: `enum`
- Default: `"off"`
- Sources:
  - CLI: `--trace`
  - ENV: `HK_TRACE`
  - Git: `hk.trace`

Enables tracing spans and performance diagnostics for detailed execution analysis.

Available formats:
- `off`: No tracing (default)
- `text`: Human-readable trace output
- `json`: Machine-readable JSON trace output
- `1` or `true`: Enable text tracing (aliases)

Useful for debugging performance issues or understanding execution flow.

Example: `HK_TRACE=text hk check` to see detailed execution traces.

### `verbose`

- Type: `u8`
- Default: `0`
- Sources:
  - CLI: `--verbose`, `-v`

Controls the verbosity of output.

Can be specified multiple times to increase verbosity:
- `-v`: Basic verbose output
- `-vv`: More detailed output
- `-vvv`: Very detailed output

Example: `hk check -vv` for detailed debugging output.

### `warnings`

- Type: `list<string>`
- Sources:
  - ENV: `HK_WARNINGS`
  - Git: `hk.warnings`
  - Pkl: `warnings`

Warning tags to enable or show. Controls which warning messages are displayed during execution.

<!-- END: AUTO-GENERATED SETTINGS -->
