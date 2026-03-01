---
outline: "deep"
---

# Configuration

hk builds its effective configuration by layering sources from lowest to highest precedence:

| Precedence | Source | Scope |
|---|---|---|
| 1 (lowest) | Built-in defaults | All projects |
| 2 | [hkrc](#hkrc) (`~/.config/hk/config.pkl`, `~/.hkrc.pkl`, or `--hkrc`) | All projects (user-level) |
| 3 | [Project config](#hk-pkl) (`hk.pkl` or `hk.local.pkl`) | Single project |
| 4 | [Git config](#git-configuration) (global, then local) | Per-repo |
| 5 | [Environment variables](#settings-reference) (`HK_*`) | Per-invocation |
| 6 (highest) | [CLI flags](#settings-reference) | Per-invocation |

Higher layers override lower. For hooks and steps, layers are **additive** — hkrc can define hooks the project doesn't have, but the project's definition wins on collision. See [hkrc merge semantics](#hkrc) for details.

## `hk.pkl`

hk is configured via `hk.pkl` which is written in [pkl-lang](https://pkl-lang.org/) from Apple.

### Config File Paths

hk searches for config files in the following order (first match wins):

| Precedence | Path | Purpose |
|---|---|---|
| 1 | `hk.local.pkl` | Local overrides, should not be committed to source control |
| 2 | `.config/hk.local.pkl` | Local overrides, nested under `.config/` |
| 3 | `hk.pkl` | Standard project config |
| 4 | `.config/hk.pkl` | Standard project config, nested under `.config/` |

hk walks up from the current directory to `/`, checking each directory for these files. The first file found is used.

Set [`HK_FILE`](/environment_variables#hk-file) to override the search and use a specific path.

> [!NOTE]
> Unlike mise, hk does not merge multiple config files or support `conf.d/` directories. Local overrides use Pkl's `amends` mechanism instead (see [`hk.local.pkl`](#hk-local-pkl)).

### Example

Here's a basic `hk.pkl` file:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.36.0/hk@1.36.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.36.0/hk@1.36.0#/Builtins.pkl"

local linters = new Mapping<String, Step> {
    // linters can be manually defined
    ["eslint"] {
        // the files to run the linter on, if no files are matched, the linter will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = List("*.js", "*.ts")
        // the command to run that makes no changes
        check = "eslint {{files}}"
        // the command to run that fixes the files (used by default)
        fix = "eslint --fix {{files}}"
        // optional: files matching these globs will be staged after fix modifies them
        // defaults to the step's glob when staging is enabled, so usually not needed
        // stage = List("*.js", "*.ts")
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

### `hk.local.pkl`

If `hk.local.pkl` exists, it will be used instead of `hk.pkl`. It is intended to be used for local config, and should
not be committed to source control.

It is assumed that the first line will be (`amends "./hk.pkl"`).

Example:

```pkl
amends "./hk.pkl"
import "./hk.pkl" as repo_config

hooks = (repo_config.hooks) {
    ["pre-commit"] {
        (steps) {
            ["custom-step"] = new Step {
                // ...
            }
        }
    }
}

```


<!--@include: ./gen/pkl-config.md-->

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

## Git status in conditions and templates

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


## `hkrc`

The `hkrc` is a global configuration file that allows you to customize hk's behavior across all projects. hk discovers it in this order (first match wins):

| Precedence | Path | Purpose |
|---|---|---|
| 1 | `.hkrc.pkl` (CWD) | Per-directory override |
| 2 | `~/.hkrc.pkl` | Home directory |
| 3 | `~/.config/hk/config.pkl` | XDG config directory |

Use the `--hkrc` flag to override discovery and use a specific path.

The hkrc file follows the same format as `hk.pkl` and can be used to define global hooks and linters that will be applied to all projects. This is useful for setting up consistent linting rules across multiple repositories.

Example hkrc file:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.36.0/hk@1.36.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.36.0/hk@1.36.0#/Builtins.pkl"

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

The hkrc is merged with the project configuration using "project wins" semantics:

- **Settings** (jobs, fail_fast, etc.): project config overrides hkrc values
- **Environment variables**: hkrc values are set first; project config can override them
- **Hooks/steps**: additive — hkrc can add hooks and steps the project doesn't define, but when both define the same step, the project's definition wins

### How to manage global hook preferences

**Run your own linters on every project**

Add steps to your hkrc. hk merges them into every project's hooks — steps with names the project doesn't define always run:

```pkl
// ~/.config/hk/config.pkl
amends "package://github.com/jdx/hk/releases/latest/hk#/Config.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["gitleaks"] { check = "gitleaks git --staged" }
        }
    }
}
```

**Skip steps you don't want from a project**

hkrc can't remove project steps — project wins on collision. To skip a step, use git config in that repo (persists) or an environment variable (one session):

```bash
# Skip a step permanently in this repo
git config --local hk.skipSteps "slow-linter,noisy-formatter"

# Skip for one run
HK_SKIP_STEPS=slow-linter hk run pre-commit
```

**Completely replace a project's hooks locally**

Create `hk.local.pkl` in the project root (don't commit it). It replaces `hk.pkl` entirely — redefine only what you want:

```pkl
// hk.local.pkl  (add to .gitignore)
amends "./hk.pkl"
import "./hk.pkl" as upstream

hooks = (upstream.hooks) {
    ["pre-commit"] {
        steps {
            // keep only the steps you want
            ["gitleaks"] = upstream.hooks["pre-commit"].steps["gitleaks"]
        }
    }
}
```

## Settings Reference

This section lists the configuration settings that control how hk behaves. Settings are sourced from multiple places; higher precedence overrides lower. Some list settings (e.g., `exclude`, `skip_steps`, `skip_hooks`, `hide_warnings`) use union semantics, combining values from multiple sources.

| Precedence | Source | Example |
|---|---|---|
| 1 | CLI flags | `hk check --fail-fast` |
| 2 | Environment variables (HK_*) | `HK_JOBS=8 hk check` |
| 3 | Git config (local repo) | `git config --local hk.jobs 4` |
| 4 | Git config (global/system) | `git config --global hk.failFast false` |
| 5 | Project config (hk.pkl) | `jobs = 4` in `hk.pkl` |
| 6 | User rc (hkrc) | `jobs = 4` in `~/.hkrc.pkl` or `~/.config/hk/config.pkl` |
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

User-specific defaults can be set in `~/.hkrc.pkl` or `~/.config/hk/config.pkl`:

```pkl
amends "package://github.com/jdx/hk/releases/latest/hk#/Config.pkl"

jobs = 4
fail_fast = false
exclude = List("node_modules", "dist", "build")
skip_steps = List("slow-test")
skip_hooks = List("pre-push")
```

> [!NOTE]
> Legacy hkrc files that amend `UserConfig.pkl` are still supported.

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

<!--@include: ./gen/settings-config.md-->
