---
description: One ship, many crews. Share step defaults across the frontend, Rust, and Terraform hands, plus the checks that range over the whole ship.
sourceHash: 244426a66241
---

# Monorepo: one ship, many crews

Muster the frontend, backend, and infrastructure checks into groups, each gang of hands to its own station, then post Markdown and YAML checks that keep watch over the whole ship.

**Before ye sail:** every tool the configuration calls on must be aboard, with each project's own configuration in its proper directory. This example expects `frontend/`, `backend/`, and `infrastructure/`.

<a href="/monorepo.pkl" download>Download monorepo.pkl</a> and save it as `hk.pkl`, the charts yer ship sails by.

## The charts {#configuration}

<<< @/public/monorepo.pkl

## Mind the boundaries between the gangs {#understand-the-boundaries}

Each group, a gang of hands, can hand its children common defaults such as `dir`, `prefix`, and `workspace_indicator`. A child step keeps any property it sets explicitly; a child's value replaces the group's rather than merging with it. Mind that builtins, the standing crew, may already set these properties themselves.

Groups also shape how the work is scheduled: within a group, the children can all haul at once, but the groups themselves run one after another, in order. If ye want the frontend and backend checks hauling side by side, put their steps in one mapping and use `depends` only where the order is truly required.

## Take her out {#try-it}

```sh
hk validate
hk check --all --plan
hk check --all
hk check --all --profile slow
```

The `slow` profile (the slow watch) enables the extra Cargo check. It is not enabled automatically in CI: the harbour-master never musters the slow watch on its own.

## Refit her for yer own ship {#adapt-it}

Change the `dir` values to match yer ship, strike off any components ye don't use, and read the `--plan` output, the passage plan, to verify how paths and workspaces are selected. When a component is skipped and ye didn't expect it, ask `hk check --why <step>`.

For tools that find nested packages on their own, see [workspaces](/configuration#workspaces).

## Boats with charts of their own: nested configs with `subprojects` {#nested-configs-with-subprojects}

Ye needn't chart every component in the root `hk.pkl`: each subproject can keep
its own `hk.pkl` right beside its code, a boat with charts of her own. The root config
lists the subproject directories (literal names or globs):

```pkl
// hk.pkl (repo root)
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"

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
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Builtins.pkl"

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
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Builtins.pkl"

local linters = new Mapping<String, Step> {
  ["cargo-fmt"] = Builtins.cargo_fmt
  ["cargo-clippy"] = Builtins.cargo_clippy
}

hooks {
  ["check"] { steps = linters }
  ["pre-commit"] { steps = linters }
}
```

The matching mise configuration has the quartermaster provision hk, aube, and each
component's tools in the directory where its steps run:

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

On Git 2.54+, ye'd best install the mise-aware launcher once per developer
machine, with `hk install --global --mise`. For an installation scoped to this one
ship, on any supported Git version, use `hk install --mise`.

When hk runs from the repo root, each subproject's hooks are merged in, and each
boat's hands keep to her own directory:

- Step working directories and glob matching are relative to the subdirectory, so
  `frontend/hk.pkl` only sees the cargo under `frontend/`.
- Each hand's name is prefixed with its directory (e.g. `frontend:eslint`), and that's
  the name to call with `--step` or `skip_steps`.
- A subproject's `env`, its standing orders, applies to its own steps only.
- Glob entries like `packages/*` match any directory holding an hk config file;
  directories without one are skipped.
- Hooks compose by name. Steps declared only under `check` do not automatically run
  under `pre-commit` or `fix`: each pipe calls only the hands listed for it.
- Set hook-wide settings such as `fix`, `stash`, `stage`, and `report` in the root
  config, so every subproject sails by the same rules.
- Only one level of subprojects is supported: no boats within boats.

This maps straight onto [mise monorepo config roots](https://mise.jdx.dev/tasks/monorepo.html):
the same directories that keep a `mise.toml` for the quartermaster can keep their own `hk.pkl`.

Use `hk check --all --plan` to read the passage plan, the resolved jobs, without running
any of them. For this example, the plan includes `frontend:eslint`, `frontend:prettier`,
`backend:cargo-fmt`, and `backend:cargo-clippy`.
