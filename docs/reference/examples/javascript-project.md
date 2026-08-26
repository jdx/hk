# Example: javascript-project

```pkl
/// Example configuration for a JavaScript/TypeScript project
/// * Uses prettier for formatting
/// * Uses eslint for linting
/// * Runs type checking with tsc
/// * Enables automatic fixes in pre-commit
amends "package://github.com/jdx/hk/releases/download/v1.56.1/hk@1.56.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.56.1/hk@1.56.1#/Builtins.pkl"

// Configure environment for all tools
env {
  ["NODE_ENV"] = "development"
}

// Define linters to use across hooks
local linters = new Mapping {
  ["prettier"] = (Builtins.prettier) {
    step {
      // Enable batch processing for performance
      batch = true
      // Run prettier after other formatters
      depends = List("eslint")
    }
  }
  ["eslint"] = (Builtins.eslint) {
    step { batch = true }
  }
  ["tsc"] = (Builtins.tsc) {
    step {
      // Type checking doesn't need file locking
      stomp = true
    }
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
