# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

**Build the project:**
```bash
mise run build
```

**Run tests:**
```bash
# Run all tests (Rust unit tests + bats integration tests)
mise run test

# Run only Rust tests
mise run test:cargo

# Run only bats tests
mise run test:bats

# Run a specific bats test file
mise run test:bats test/check.bats
```

**Lint and format code:**
```bash
# Run all linters and checks
hk check --all
hk check --all --slow  # includes slower checks

# Fix formatting and linting issues
hk fix --all
hk fix --all --slow

# or use mise tasks:
mise run lint
mise run lint-fix
```

**Development workflow:**
```bash
# Build and run hk in dev mode
mise run dev
```

## High-Level Architecture

hk is a git hook manager and project linting tool written in Rust with emphasis on performance and concurrent execution. The architecture leverages file locks to maximize concurrency while preventing race conditions.

### Core Components

**Configuration System (src/config.rs):**
- Supports multiple config formats: `.pkl` (primary), `.toml`, `.yaml`, `.json`
- Main config file: `hk.pkl` in project root
- Uses Pkl (github.com/apple/pkl) as the primary configuration language
- Config amends a base schema from `pkl/Config.pkl`

**Hook System (src/hook.rs):**
- Manages git hooks (pre-commit, pre-push, commit-msg, prepare-commit-msg)
- Supports custom hooks like "check" and "fix" for manual runs
- Implements stashing strategies for git hooks
- Handles concurrent step execution with proper locking

**Step Execution (src/step.rs, src/step_job.rs):**
- Steps are individual linting/formatting tasks
- Each step can have: check, fix, shell commands
- Steps support glob patterns for file filtering
- Steps can depend on other steps
- Steps use read/write file locks to prevent conflicts

**File Locking (src/file_rw_locks.rs):**
- Implements a sophisticated file locking system
- Allows multiple readers or single writer per file
- Prevents race conditions during concurrent execution
- Critical for maximizing parallelism

**Built-in Linters (pkl/builtins/):**
- Extensive library of pre-configured linters and formatters
- Each builtin is a Pkl file defining step configuration
- Used via `Builtins.linter_name` in hk.pkl

**CLI Interface (src/cli/):**
- Subcommands: init, install, uninstall, check, fix, run, validate, config
- Uses clap for argument parsing
- Supports running specific hooks or steps

### Key Design Patterns

1. **Concurrent Execution:** Steps run in parallel when possible, using tokio for async runtime
2. **File-based Coordination:** Uses file locks instead of in-memory coordination for cross-process safety
3. **Pluggable Configuration:** Pkl-based config allows easy extension and customization
4. **Progressive Enhancement:** Works with or without git, libgit2, mise, etc.

### Integration Points

- **Git Integration:** Can use either libgit2 or shell git commands (controlled by HK_LIBGIT2 env var)
- **Mise Integration:** Deeply integrated with mise for task running and tool management
- **Tool Discovery:** Automatically finds tools via PATH or mise shims
