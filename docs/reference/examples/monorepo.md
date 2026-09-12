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
amends "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Config.pkl"

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
amends "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Builtins.pkl"

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
amends "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Builtins.pkl"

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
# mise.toml (repo root)
monorepo_root = true

[monorepo]
config_roots = [".", "frontend", "backend"]

[tools]
aube = "latest"
hk = "latest"

[env]
HK_MISE = 1

[hooks]
postinstall = "hk install --mise"
```

```toml
# frontend/mise.toml
[tools]
node = "lts"
```

```toml
# backend/mise.toml
[tools]
rust = "stable"
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
- Hooks compose by name. Steps declared only under `check` do not automatically run
  under `pre-commit` or `fix`.
- Define hook-wide settings such as `fix`, `stash`, `stage`, and `report` in the root
  config so every subproject uses the same behavior.
- Only one level of subprojects is supported.

This maps directly onto [mise monorepo config roots](https://mise.jdx.dev/tasks/monorepo.html):
the same directories that own a `mise.toml` can own their `hk.pkl`.

Use `hk check --all --plan` to inspect the resolved jobs without executing them.
For this example, the plan includes `frontend:eslint`, `frontend:prettier`,
`backend:cargo-fmt`, and `backend:cargo-clippy`.
