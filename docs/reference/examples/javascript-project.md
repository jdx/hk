# Example: javascript-project

```pkl
/// Example configuration for a JavaScript/TypeScript project
/// * Uses prettier for formatting
/// * Uses eslint for linting  
/// * Runs type checking with tsc
/// * Enables automatic fixes in pre-commit

amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Builtins.pkl"

// Configure environment for all tools
env {
  ["NODE_ENV"] = "development"
}

// Define linters to use across hooks
local linters = new Mapping<String, Step> {
  ["prettier"] = (Builtins.prettier) {
    // Enable batch processing for performance
    batch = true
    // Run prettier after other formatters
    depends = List("eslint")
  }
  ["eslint"] = (Builtins.eslint) {
    batch = true
  }
  ["tsc"] = (Builtins.tsc) {
    // Type checking doesn't need file locking
    stomp = true
  }
}

hooks {
  ["pre-commit"] {
    // Enable automatic fixes
    fix = true
    // Stash unstaged changes
    stash = "git"
    steps = linters
  }
  ["pre-push"] {
    // Just check, don't fix
    steps = linters
  }
  ["check"] {
    steps = linters
  }
  ["fix"] {
    fix = true
    steps = linters
  }
}
```

## Description

Example configuration for a JavaScript/TypeScript project
* Uses prettier for formatting
* Uses eslint for linting  
* Runs type checking with tsc
* Enables automatic fixes in pre-commit

## Key Features

- Standard configuration

