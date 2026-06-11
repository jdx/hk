# Example: monorepo

This example shows a monorepo with frontend, backend, infrastructure, and shared steps.

## Original style

Before group-level step defaults, repeated settings such as `dir` and `workspace_indicator` had to be set on each child step.

```pkl
/// Example configuration for a monorepo with multiple languages
/// * Frontend: JavaScript/TypeScript with React
/// * Backend: Rust
/// * Infrastructure: Terraform
/// * Uses groups to organize steps by component

amends "package://github.com/jdx/hk/releases/download/v1.47.0/hk@1.47.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.47.0/hk@1.47.0#/Builtins.pkl"

// Frontend linters (JavaScript/TypeScript)
local frontend = new Group {
  steps {
    ["prettier"] = (Builtins.prettier) {
      dir = "frontend"
      batch = true
    }
    ["eslint"] = (Builtins.eslint) {
      dir = "frontend"
      batch = true
    }
    ["stylelint"] = (Builtins.stylelint) {
      glob = List("frontend/**/*.css", "frontend/**/*.scss")
    }
  }
}

// Backend linters (Rust)
local backend = new Group {
  steps {
    ["cargo_fmt"] = (Builtins.cargo_fmt) {
      workspace_indicator = "Cargo.toml"
      dir = "backend"
    }
    ["cargo_clippy"] = (Builtins.cargo_clippy) {
      workspace_indicator = "Cargo.toml"
      dir = "backend"
    }
    ["cargo_check"] = (Builtins.cargo_check) {
      dir = "backend"
      // Only run in CI or with "full" profile
      profiles = List("ci", "full")
    }
  }
}

// Infrastructure linters (Terraform)
local infrastructure = new Group {
  steps {
    ["terraform"] = (Builtins.terraform) {
      glob = List("infrastructure/**/*.tf")
    }
    ["tflint"] = (Builtins.tf_lint) {
      glob = List("infrastructure/**/*.tf")
    }
  }
}

// Shared linters (apply to all components)
local shared = new Mapping<String, Step> {
  ["markdown"] = (Builtins.markdown_lint) {
    glob = List("**/*.md")
    exclude = List("**/node_modules/**", "**/target/**")
  }
  ["yaml"] = (Builtins.yamllint) {
    glob = List("**/*.yaml", "**/*.yml")
    exclude = List("**/node_modules/**")
  }
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["frontend"] = frontend
      ["backend"] = backend
      ["infrastructure"] = infrastructure
      ...shared
    }
  }
  ["check"] {
    steps {
      ["frontend"] = frontend
      ["backend"] = backend
      ["infrastructure"] = infrastructure
      ...shared
    }
  }
}
```

## Simplified with group options

Group-level `dir` and `workspace_indicator` remove repeated child step settings. A child step can still override any inherited field by setting its own value.

```pkl
/// Example configuration for a monorepo with multiple languages
/// * Frontend: JavaScript/TypeScript with React
/// * Backend: Rust
/// * Infrastructure: Terraform
/// * Uses groups to organize steps by component

amends "package://github.com/jdx/hk/releases/download/v1.47.0/hk@1.47.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.47.0/hk@1.47.0#/Builtins.pkl"

// Frontend linters (JavaScript/TypeScript)
local frontend = new Group {
  dir = "frontend"
  steps {
    ["prettier"] = (Builtins.prettier) {
      batch = true
    }
    ["eslint"] = (Builtins.eslint) {
      batch = true
    }
    ["stylelint"] = (Builtins.stylelint) {
      glob = List("**/*.css", "**/*.scss")
    }
  }
}

// Backend linters (Rust)
local backend = new Group {
  dir = "backend"
  workspace_indicator = "Cargo.toml"
  steps {
    ["cargo_fmt"] = Builtins.cargo_fmt
    ["cargo_clippy"] = Builtins.cargo_clippy
    ["cargo_check"] = (Builtins.cargo_check) {
      // Only run in CI or with "full" profile
      profiles = List("ci", "full")
    }
  }
}

// Infrastructure linters (Terraform)
local infrastructure = new Group {
  dir = "infrastructure"
  steps {
    ["terraform"] = (Builtins.terraform) {
      glob = "**/*.tf"
    }
    ["tflint"] = (Builtins.tf_lint) {
      glob = "**/*.tf"
    }
  }
}

// Shared linters (apply to all components)
local shared = new Mapping<String, Step> {
  ["markdown"] = (Builtins.markdown_lint) {
    glob = List("**/*.md")
    exclude = List("**/node_modules/**", "**/target/**")
  }
  ["yaml"] = (Builtins.yamllint) {
    glob = List("**/*.yaml", "**/*.yml")
    exclude = List("**/node_modules/**")
  }
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["frontend"] = frontend
      ["backend"] = backend
      ["infrastructure"] = infrastructure
      ...shared
    }
  }
  ["check"] {
    steps {
      ["frontend"] = frontend
      ["backend"] = backend
      ["infrastructure"] = infrastructure
      ...shared
    }
  }
}
```

## Description

Example configuration for a monorepo with multiple languages
* Frontend: JavaScript/TypeScript with React
* Backend: Rust
* Infrastructure: Terraform
* Uses groups to organize steps by component

## Key Features

- Standard configuration
