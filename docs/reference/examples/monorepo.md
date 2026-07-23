# Example: monorepo

This example shows a monorepo with frontend, backend, infrastructure, and shared steps.

Groups can set common step attributes such as `dir`, `workspace_indicator`, `prefix`, `shell`, `stage`, and `exclude`. Child steps inherit those values by default, but a child can still set its own value when it needs different behavior. Child values replace group values; they are not merged.

## Configuration

```pkl
/// Example configuration for a monorepo with multiple languages
/// * Frontend: JavaScript/TypeScript with React
/// * Backend: Rust
/// * Infrastructure: Terraform
/// * Uses groups to organize steps by component

amends "package://github.com/jdx/hk/releases/download/v1.53.0/hk@1.53.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.53.0/hk@1.53.0#/Builtins.pkl"

// Frontend linters (JavaScript/TypeScript)
local frontend = new Group {
  // Inherited by frontend steps unless a child overrides `dir`.
  dir = "frontend"
  steps {
    ["prettier"] = (Builtins.prettier) {
      batch = true
    }
    ["eslint"] = (Builtins.eslint) {
      batch = true
    }
    ["stylelint"] = (Builtins.stylelint) {
      // Override the group dir for a step that scans files from the repo root.
      dir = "."
      glob = List("frontend/**/*.css", "frontend/**/*.scss", "packages/design-system/**/*.scss")
    }
  }
}

// Backend linters (Rust)
local backend = new Group {
  // Inherited by all backend steps.
  dir = "backend"
  workspace_indicator = "Cargo.toml"
  steps {
    ["cargo_fmt"] = Builtins.cargo_fmt
    ["cargo_clippy"] = Builtins.cargo_clippy
    ["cargo_check"] = (Builtins.cargo_check) {
      // Only run in CI or with "full" profile.
      profiles = List("ci", "full")
    }
  }
}

// Infrastructure linters (Terraform)
local infrastructure = new Group {
  dir = "infrastructure"
  exclude = List("**/.terraform/**")
  steps {
    ["terraform"] = (Builtins.terraform) {
      glob = "**/*.tf"
    }
    ["tflint"] = (Builtins.tf_lint) {
      glob = "**/*.tf"
      // Child exclude replaces the group exclude, so repeat common exclusions.
      exclude = List("**/.terraform/**", "modules/vendor/**")
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

## Key Features

- Group-level defaults keep shared settings close to the component they apply to.
- Child steps can override inherited values when a tool needs a different working directory, glob, shell, stage, prefix, workspace indicator, or exclude list.
- Override semantics are simple: a child value replaces the group value instead of merging with it.

## Nested configs with `subprojects`

Instead of describing every component in the root `hk.pkl`, each subproject can own
its own `hk.pkl` next to its code. The root config lists the subproject directories
(literals or globs):

```pkl
// hk.pkl (repo root)
amends "package://github.com/jdx/hk/releases/download/v1.53.0/hk@1.53.0#/Config.pkl"

subprojects = List("frontend", "backend", "packages/*")

hooks {
  ["check"] {}
  ["pre-commit"] { fix = true }
}
```

```pkl
// frontend/hk.pkl
amends "package://github.com/jdx/hk/releases/download/v1.53.0/hk@1.53.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.53.0/hk@1.53.0#/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["eslint"] = (Builtins.eslint) { batch = true }
      ["prettier"] = (Builtins.prettier) { batch = true }
    }
  }
}
```

When hk runs from the repo root, each subproject's hooks are merged in, scoped to
its directory:

- Step working directories and glob matching are relative to the subdirectory, so
  `frontend/hk.pkl` only sees files under `frontend/`.
- Step names are prefixed with the directory (e.g. `frontend:eslint`), which is the
  name to use with `--step` or `skip_steps`.
- A subproject's `env` applies to its own steps only.
- Glob entries like `packages/*` match any directory containing an hk config file;
  directories without one are skipped.

This maps directly onto [mise monorepo config roots](https://mise.jdx.dev/tasks/monorepo.html):
the same directories that own a `mise.toml` can own their `hk.pkl`.
