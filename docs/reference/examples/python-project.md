# Example: python-project

```pkl
/// Example configuration for a Python project
/// * Uses ruff for fast linting
/// * Uses ruff_format for fast formatting
/// * Uses mypy for type checking
/// * Sorts imports with isort
/// * Validates with flake8

amends "package://github.com/jdx/hk/releases/download/vtest/hk@test#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/vtest/hk@test#/Builtins.pkl"

local python_linters = new Mapping<String, Step> {
  // Ruff is a fast Python linter
  ["ruff"] = (Builtins.ruff) {
    // Run ruff first as it's the fastest
    batch = true
  }
  // Ruff formatter for code formatting
  ["ruff_format"] = (Builtins.ruff_format) {
    depends = "ruff"  // Run after ruff
  }
  // Black for consistent formatting (alternative to ruff_format)
  ["black"] = (Builtins.black) {
    depends = "ruff_format"  // Run after ruff_format
  }
  // isort for import sorting
  ["isort"] = (Builtins.isort) {
    depends = "black"  // Run after black
  }
  // Type checking with mypy
  ["mypy"] = (Builtins.mypy) {
    // Type checking doesn't modify files
    stomp = true
    // Only run with "types" profile
    profiles = List("types", "full")
  }
  // Additional validation with flake8
  ["flake8"] = (Builtins.flake8) {
    // Only run in CI
    profiles = List("ci")
  }
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = python_linters
  }
  ["pre-push"] {
    // Include type checking on push
    steps = python_linters
    env {
      ["HK_PROFILES"] = "types"
    }
  }
  ["check"] {
    steps = python_linters
  }
  ["fix"] {
    fix = true
    steps = python_linters
  }
}

// Enable fail-fast for quicker feedback
fail_fast = true
```

## Description

Example configuration for a Python project
* Uses ruff for fast linting
* Uses ruff_format for fast formatting
* Uses mypy for type checking
* Sorts imports with isort
* Validates with flake8

## Key Features

- Standard configuration

