---
description: Share step defaults across frontend, Rust, Terraform, and repository-wide checks.
---

# Monorepo

Organize frontend, backend, and infrastructure checks into groups, then add repository-wide Markdown and YAML checks.

**Prerequisites:** the tools referenced in the configuration must be available, with project configuration in the appropriate directories. This example expects `frontend/`, `backend/`, and `infrastructure/`.

<a href="/monorepo.pkl" download>Download monorepo.pkl</a> and save it as `hk.pkl`.

## Configuration

<<< @/public/monorepo.pkl

## Understand the boundaries

Each group can provide common defaults such as `dir`, `prefix`, and `workspace_indicator`. A child keeps an explicitly defined property; child values replace rather than merge with group values. Builtins may already define these properties.

Groups also affect scheduling: children can run concurrently within a group, but groups run in order. If frontend and backend checks should overlap, place the steps in one mapping and use `depends` only where ordering is required.

## Try it

```sh
hk validate
hk check --all --plan
hk check --all
hk check --all --profile slow
```

The `slow` profile enables the additional Cargo check. It is not enabled automatically in CI.

## Adapt it

Change `dir` values to match your repository, remove components you do not use, and inspect `--plan` to verify how paths and workspaces are selected. Use `hk check --why <step>` when a component is unexpectedly skipped.

For tools that discover nested packages, see [workspaces](/configuration#workspaces).

## Nested configs with `subprojects`

Instead of describing every component in the root `hk.pkl`, each subproject can own
its own `hk.pkl` next to its code. The root config lists the subproject directories
(literals or globs):

```pkl
// hk.pkl (repo root)
amends "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Config.pkl"

subprojects = List("frontend", "backend", "packages/*")

hooks {
  ["check"] {}
  ["pre-commit"] {
    fix = true
    stash = "git"
  }
}
```

```pkl
// frontend/hk.pkl
amends "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Builtins.pkl"

local linters = new Mapping<String, Step> {
  // aube resolves these executables from frontend/node_modules/.bin
  ["eslint"] = (Builtins.eslint) {
    prefix = List("aube", "exec")
  }
  ["prettier"] = (Builtins.prettier) {
    prefix = List("aube", "exec")
  }
}

hooks {
  ["check"] {
    steps = linters
  }
  // Hooks compose by name, so list the steps again for pre-commit.
  ["pre-commit"] {
    steps = linters
  }
}
```

```pkl
// backend/hk.pkl
amends "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Builtins.pkl"

local linters = new Mapping<String, Step> {
  ["cargo-fmt"] = Builtins.cargo_fmt
  ["cargo-clippy"] = Builtins.cargo_clippy
}

hooks {
  ["check"] { steps = linters }
  ["pre-commit"] { steps = linters }
}
```

The matching mise configuration makes hk, aube, and each component's tools
available in the directory where its steps run:

```toml

```
