# Example: monorepo

```pkl
/// Example configuration for a monorepo with multiple languages
/// * Frontend: JavaScript/TypeScript with React
/// * Backend: Rust
/// * Infrastructure: Terraform
/// * Uses groups to organize steps by component

amends "package://github.com/jdx/hk/releases/download/v1.29.0/hk@1.29.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.29.0/hk@1.29.0#/Builtins.pkl"

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

## Description

Example configuration for a monorepo with multiple languages
* Frontend: JavaScript/TypeScript with React
* Backend: Rust
* Infrastructure: Terraform
* Uses groups to organize steps by component

## Key Features

- Standard configuration

