---
outline: "deep"
---

# Configuration Schema Reference

This document provides a complete reference for the `hk.pkl` configuration schema. hk uses [Pkl](https://pkl-lang.org) for configuration, providing type safety, validation, and excellent IDE support.

## Basic Structure

Every `hk.pkl` file must start with:

```pkl
amends "package://github.com/jdx/hk/releases/download/v{{version}}/hk@{{version}}#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v{{version}}/hk@{{version}}#/Builtins.pkl"
```

## Top-Level Configuration

### `min_hk_version`
- **Type:** `String`
- **Default:** `"{{version | truncate(length=1)}}.0.0"`
- **Description:** Minimum required hk version for this configuration

### `default_branch`
- **Type:** `String?` (optional)
- **Default:** Auto-detected
- **Description:** Preferred default branch to compare against (e.g., "main", "origin/main")
- **Example:**
  ```pkl
  default_branch = "main"
  ```

### `fail_fast`
- **Type:** `Boolean?` (optional)
- **Default:** `true`
- **Description:** Abort remaining steps/groups after the first failure
- **Example:**
  ```pkl
  fail_fast = false  // Continue running all steps even if one fails
  ```

### `display_skip_reasons`
- **Type:** `List<String>`
- **Default:** `List("profile-not-enabled")`
- **Description:** Controls which skip reasons are displayed in the output
- **Available values:**
  - `"profile-not-enabled"` - Profile not active (default)
  - `"profile-explicitly-disabled"` - Profile explicitly disabled
  - `"no-command-for-run-type"` - No command available for the run type
  - `"no-files-to-process"` - No files matched the glob patterns
  - `"condition-false"` - Condition evaluated to false
  - `"disabled-by-env"` - Disabled by environment variable
  - `"disabled-by-cli"` - Disabled by CLI flag
- **Example:**
  ```pkl
  display_skip_reasons = List("profile-not-enabled", "no-files-to-process")
  ```

### `warnings`
- **Type:** `List<String>`
- **Default:** `List()` (empty - no warnings shown)
- **Description:** Which warning categories to show
- **Available tags:**
  - `"missing-profiles"` - Warn about missing profile definitions
- **Example:**
  ```pkl
  warnings = List("missing-profiles")
  ```

### `exclude`
- **Type:** `String | List<String>?` (optional)
- **Default:** `List()` (empty - no files excluded)
- **Description:** Global exclude patterns applied to all hooks and steps
- **Example:**
  ```pkl
  exclude = "*.test.js"  // Single pattern as string
  exclude = List("*.test.js", "node_modules", "dist")  // Multiple patterns
  ```

### `env`
- **Type:** `Mapping<String, String>`
- **Default:** Empty mapping
- **Description:** Environment variables for hk and linters
- **Example:**
  ```pkl
  env {
    ["NODE_ENV"] = "production"
    ["HK_FAIL_FAST"] = "0"
  }
  ```

### `hooks`
- **Type:** `Mapping<String, Hook>`
- **Description:** Git hooks and custom hooks configuration
- **Example:**
  ```pkl
  hooks {
    ["pre-commit"] { ... }
    ["pre-push"] { ... }
    ["check"] { ... }
    ["fix"] { ... }
  }
  ```

## Hook Configuration

Each hook in the `hooks` mapping has the following properties:

### `fix`
- **Type:** `Boolean?` (optional)
- **Description:** Whether to run fix commands (modifies files) or check commands (read-only)
- **Default:** `false`
- **Example:**
  ```pkl
  hooks {
    ["pre-commit"] {
      fix = true  // Enable file modifications
    }
  }
  ```

### `stash`
- **Type:** `StashMethod` = `Boolean | "git" | "patch-file" | "none"`
- **Description:** How to handle unstaged changes before running fix steps
- **Values:**
  - `true` or `"git"` - Use git stash (default for pre-commit with fix=true)
  - `"patch-file"` - Create a patch file
  - `false` or `"none"` - No stashing
- **Example:**
  ```pkl
  hooks {
    ["pre-commit"] {
      stash = "patch-file"  // Use patch file instead of git stash
    }
  }
  ```

### `report`
- **Type:** `String | Script?` (optional)
- **Description:** Command to run after hook completion. Receives timing JSON in `HK_REPORT_JSON`
- **Example:**
  ```pkl
  hooks {
    ["check"] {
      report = #"node scripts/upload-timings.js <<<"$HK_REPORT_JSON""#
    }
  }
  ```

### `steps`
- **Type:** `Mapping<String, Step | Group>`
- **Description:** Steps or groups to run in this hook
- **Example:**
  ```pkl
  hooks {
    ["pre-commit"] {
      steps {
        ["prettier"] = Builtins.prettier
        ["eslint"] = Builtins.eslint
      }
    }
  }
  ```

## Step Configuration

Steps define individual linting/formatting tasks. Each step has these properties:

### File Selection

#### `glob`
- **Type:** `String | List<String>?` (optional)
- **Description:** File patterns to include
- **Example:**
  ```pkl
  glob = "*.js"  // Single pattern
  glob = List("*.js", "*.ts", "**/*.jsx")  // Multiple patterns
  ```

#### `exclude`
- **Type:** `String | List<String>?` (optional)
- **Description:** File patterns to exclude
- **Example:**
  ```pkl
  exclude = List("**/node_modules/**", "dist/**")
  ```

#### `stage`
- **Type:** `String | List<String>?` (optional)
- **Description:** Files to stage after running fix step
- **Special Values:**
  - `"<JOB_FILES>"` - Stage only the files that were actually processed by this step (after `check_list_files` filtering). Use this when `check_list_files` dynamically discovers files to avoid staging unrelated files that match the glob.
- **Examples:**
  ```pkl
  stage = glob  // Stage the same files that were processed
  stage = "<JOB_FILES>"  // Stage only files processed after check_list_files filtering
  stage = List("*.js", "*.ts")  // Stage specific patterns
  ```
- **Use case for `<JOB_FILES>`:**
  When using `check_list_files` with a broad glob like `**/*`, you want to stage only the files that were actually processed, not all files matching the glob:
  ```pkl
  ["shfmt"] {
    glob = "**/*"
    stage = "<JOB_FILES>"  // Only stage shell scripts, not all files
    check_list_files = "find . -name '*.sh' -type f"
    fix = "shfmt -w {{files}}"
  }
  ```

### Commands

#### `check`
- **Type:** `String | Script?` (optional)
- **Description:** Command that validates files without modifications
- **Template variables:** `{{files}}` - Space-separated file list
- **Example:**
  ```pkl
  check = "eslint {{files}}"
  ```

#### `fix`
- **Type:** `String | Script?` (optional)
- **Description:** Command that modifies files in place
- **Example:**
  ```pkl
  fix = "eslint --fix {{files}}"
  ```

#### `check_list_files`
- **Type:** `String | Script?` (optional)
- **Description:** Command that outputs list of files needing fixes (optimizes check_first)
- **Example:**
  ```pkl
  check_list_files = "prettier --list-different {{files}}"
  ```

#### `check_diff`
- **Type:** `String | Script?` (optional)
- **Description:** Command that outputs a diff/patch that hk can apply
- **Example:**
  ```pkl
  check_diff = "cd {{workspace}} && go mod tidy -diff"
  ```

### Execution Control

#### `exclusive`
- **Type:** `Boolean`
- **Default:** `false`
- **Description:** Run step in isolation, preventing parallel execution
- **Example:**
  ```pkl
  exclusive = true  // Wait for other steps to finish, then run alone
  ```

#### `interactive`
- **Type:** `Boolean`
- **Default:** `false`
- **Description:** Connect stdin/stdout/stderr to hk's execution (implies exclusive)
- **Example:**
  ```pkl
  interactive = true  // For prompts or interactive tools
  ```

#### `depends`
- **Type:** `String | List<String>`
- **Default:** `List()`
- **Description:** Wait for specific sibling steps to finish first
- **Example:**
  ```pkl
  depends = "build"  // Wait for "build" step
  depends = List("lint", "format")  // Wait for multiple steps
  ```

#### `check_first`
- **Type:** `Boolean`
- **Default:** `true`
- **Description:** Run check before fix when multiple steps target the same files
- **Example:**
  ```pkl
  check_first = false  // Always run fix directly
  ```

#### `batch`
- **Type:** `Boolean`
- **Default:** `false`
- **Description:** Process files in batches for parallel execution
- **Example:**
  ```pkl
  batch = true  // Split files into batches for parallel processing
  ```

#### `stomp`
- **Type:** `Boolean`
- **Default:** `false`
- **Description:** Use read locks instead of write locks for fix commands
- **Example:**
  ```pkl
  stomp = true  // Allow concurrent writes (tool has own locking)
  ```

### Workspace Support

#### `workspace_indicator`
- **Type:** `String?` (optional)
- **Description:** Filename indicating a workspace root (e.g., "Cargo.toml", "package.json")
- **Template variable:** `{{workspace}}` - Directory containing the indicator file
- **Example:**
  ```pkl
  workspace_indicator = "Cargo.toml"
  check = "cargo clippy --manifest-path {{workspace_indicator}}"
  ```

### Environment & Execution

#### `shell`
- **Type:** `String | Script?` (optional)
- **Description:** Shell to use for commands
- **Example:**
  ```pkl
  shell = "/bin/bash"
  ```

#### `prefix`
- **Type:** `String?` (optional)
- **Description:** Command prefix (e.g., "mise exec --", "npm run")
- **Example:**
  ```pkl
  prefix = "mise exec --"
  ```

#### `dir`
- **Type:** `String?` (optional)
- **Description:** Working directory for commands
- **Example:**
  ```pkl
  dir = "frontend"
  ```

#### `condition`
- **Type:** `String?` (optional)
- **Description:** Shell command that must succeed for step to run
- **Example:**
  ```pkl
  condition = "test -f package.json"
  ```

#### `env`
- **Type:** `Mapping<String, String>`
- **Default:** Empty mapping
- **Description:** Step-specific environment variables
- **Example:**
  ```pkl
  env {
    ["NODE_ENV"] = "test"
  }
  ```

### Profile Support

#### `profiles`
- **Type:** `List<String>?` (optional)
- **Description:** Which profiles (HK_PROFILES) must be active for step to run
- **Example:**
  ```pkl
  profiles = List("backend", "slow")
  ```

### Output Control

#### `hide`
- **Type:** `Boolean`
- **Default:** `false`
- **Description:** Hide step from output
- **Example:**
  ```pkl
  hide = true  // Don't show this step in output
  ```

#### `output_summary`
- **Type:** `"stdout" | "stderr" | "combined" | "hide"`
- **Default:** `"stderr"`
- **Description:** Which stream(s) to include in end-of-run summary
- **Example:**
  ```pkl
  output_summary = "combined"  // Show both stdout and stderr
  ```

### Testing

#### `tests`
- **Type:** `Mapping<String, StepTest>`
- **Default:** Empty mapping
- **Description:** Per-step tests runnable via `hk test`
- **Example:**
  ```pkl
  tests {
    ["formats json"] {
      run = "fix"
      write { ["{{tmp}}/a.json"] = #"{"b":1}"# }
      files = List("{{tmp}}/a.json")
      expect {
        files { ["{{tmp}}/a.json"] = #"{ "b": 1 }\n"# }
      }
    }
  }
  ```

## Script Type

The `Script` type allows platform-specific commands:

```pkl
class Script {
  linux: String?    // Command for Linux
  macos: String?    // Command for macOS
  windows: String?  // Command for Windows
  other: String?    // Fallback for other platforms
}
```

Example:
```pkl
check = new Script {
  linux = "linux-linter {{files}}"
  macos = "mac-linter {{files}}"
  windows = "windows-linter.exe {{files}}"
}
```

## Group Configuration

Groups allow organizing steps hierarchically:

```pkl
class Group {
  steps: Mapping<String, Step>
}
```

Example:
```pkl
hooks {
  ["pre-commit"] {
    steps {
      ["frontend"] = new Group {
        steps {
          ["prettier"] = Builtins.prettier
          ["eslint"] = Builtins.eslint
        }
      }
      ["backend"] = new Group {
        steps {
          ["cargo_fmt"] = Builtins.cargo_fmt
          ["cargo_clippy"] = Builtins.cargo_clippy
        }
      }
    }
  }
}
```

## StepTest Configuration

Tests allow validating step behavior:

### `run`
- **Type:** `"check" | "fix"`
- **Default:** `"check"`
- **Description:** Which command to test

### `files`
- **Type:** `String | List<String>?`
- **Description:** Files to pass to the command

### `fixture`
- **Type:** `String?`
- **Description:** Path to copy into temp sandbox

### `write`
- **Type:** `Mapping<String, String>`
- **Description:** Files to create in sandbox
- **Template variable:** `{{tmp}}` - Temp directory path

### `before`
- **Type:** `String?`
- **Description:** Command to run before test

### `after`
- **Type:** `String?`
- **Description:** Command to run after test

### `env`
- **Type:** `Mapping<String, String>`
- **Description:** Test-specific environment variables

### `expect`
- **Type:** `StepTestExpect`
- **Properties:**
  - `code: Int` - Expected exit code (default 0)
  - `stdout: String?` - Expected stdout substring
  - `stderr: String?` - Expected stderr substring
  - `files: Mapping<String, String>` - Expected file contents

## Template Variables

Commands support these template variables:

- `{{files}}` - Space-separated list of matched files
- `{{workspace}}` - Directory containing workspace_indicator file
- `{{workspace_indicator}}` - Path to workspace indicator file
- `{{tmp}}` - Temporary directory (in tests)

## Built-in Linters

hk provides 60+ pre-configured linters via the `Builtins` module. See the [full list](../builtins.md) with examples.

Common builtins:
- `Builtins.prettier` - JavaScript/TypeScript/CSS formatter
- `Builtins.eslint` - JavaScript/TypeScript linter
- `Builtins.cargo_fmt` - Rust formatter
- `Builtins.cargo_clippy` - Rust linter
- `Builtins.black` - Python formatter
- `Builtins.ruff` - Python linter
- `Builtins.shellcheck` - Shell script linter
- `Builtins.shfmt` - Shell script formatter

## Complete Examples

### Basic JavaScript/TypeScript Project

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Builtins.pkl"

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["prettier"] = Builtins.prettier
      ["eslint"] = Builtins.eslint
    }
  }
  ["check"] {
    steps {
      ["prettier"] = Builtins.prettier
      ["eslint"] = Builtins.eslint
      ["tsc"] = Builtins.tsc
    }
  }
}
```

### Monorepo with Multiple Languages

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Builtins.pkl"

local frontend_linters = new Mapping<String, Step> {
  ["prettier"] = (Builtins.prettier) {
    dir = "frontend"
  }
  ["eslint"] = (Builtins.eslint) {
    dir = "frontend"
    batch = true
  }
}

local backend_linters = new Mapping<String, Step> {
  ["cargo_fmt"] = (Builtins.cargo_fmt) {
    workspace_indicator = "Cargo.toml"
  }
  ["cargo_clippy"] = (Builtins.cargo_clippy) {
    workspace_indicator = "Cargo.toml"
  }
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["frontend"] = new Group { steps = frontend_linters }
      ["backend"] = new Group { steps = backend_linters }
    }
  }
}
```

### Custom Linter with Platform-Specific Commands

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Config.pkl"

hooks {
  ["pre-commit"] {
    steps {
      ["custom-linter"] {
        glob = List("*.custom")
        check = new Script {
          linux = "custom-linter-linux {{files}}"
          macos = "custom-linter-mac {{files}}"
          windows = "custom-linter.exe {{files}}"
        }
        fix = new Script {
          linux = "custom-linter-linux --fix {{files}}"
          macos = "custom-linter-mac --fix {{files}}"
          windows = "custom-linter.exe /fix {{files}}"
        }
      }
    }
  }
}
```

### Step with Dependencies and Conditions

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Builtins.pkl"

hooks {
  ["pre-commit"] {
    steps {
      ["build"] {
        check = "npm run build"
        exclusive = true
      }
      ["test"] {
        check = "npm test"
        depends = "build"
        condition = "test -f package.json"
      }
      ["prettier"] = (Builtins.prettier) {
        depends = List("build", "test")
      }
    }
  }
}
```

### Profile-Based Configuration

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.21.1/hk@1.21.1#/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["quick-check"] = (Builtins.eslint) {
        profiles = List("quick")
      }
      ["full-check"] = (Builtins.eslint) {
        profiles = List("full")
        check = "eslint --max-warnings 0 {{files}}"
      }
      ["security-scan"] {
        profiles = List("security", "full")
        check = "npm audit"
        exclusive = true
      }
    }
  }
}
```

Run with profiles:
```bash
HK_PROFILES=quick hk check      # Only quick-check
HK_PROFILES=full hk check       # full-check and security-scan
HK_PROFILES=security hk check   # Only security-scan
```

## See Also

- [Built-in Linters Reference](../builtins.md)
- [Configuration Examples](examples/index.md)
- [Pkl Language Documentation](https://pkl-lang.org/main/current/language.html)
- [Configuration Guide](../configuration.md)
- [Environment Variables](../environment_variables.md)
