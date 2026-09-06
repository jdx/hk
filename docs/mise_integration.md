---
description: Use mise to provide hk and linter versions, environment variables, and Git hook execution.
---

# mise integration

[mise](https://mise.jdx.dev/) manages tools, environments, and tasks. hk selects files and coordinates checks and fixes. Together they let a team share tool versions and run hooks from terminals, editors, and CI.

mise is optional: hk can run any executable available on `PATH`.

## Install tools

From your project directory:

```sh
mise use hk
mise use npm:prettier
```

Commit the resulting `mise.toml`. Other developers can run `mise install` to install the declared versions.

For tools already managed by a language package manager, keep them there. For example, expose a Node project’s installed executables through mise:

```toml
[env]
_.path = ["node_modules/.bin"]
```

Run the project’s package installation command before invoking hk. See [mise tool management](https://mise.jdx.dev/dev-tools/) for supported backends.

## Make tools available to Git

Install hk’s hooks with mise integration:

```sh
hk install --mise
```

Or, on Git 2.54+, install globally:

```sh
hk install --global --mise
```

The launcher uses `mise x` to prepare the project environment before running hk. Developers do not need an activated shell, but Git must be able to find `mise` itself.

Setting `HK_MISE=1` makes `--mise` the default for later `hk init` and `hk install` commands. It does not rewrite an already-installed launcher until installation runs again.

## Generate a starter setup

`hk init --mise` creates `hk.pkl` and, when absent, a `mise.toml` with hk configured and a `pre-commit` task.

```sh
hk init --mise
hk install --mise
```

Review the generated tools and tasks. Existing `mise.toml` files are preserved.

## Install hooks when tools are installed

Add a postinstall hook to the project’s `mise.toml`:

```toml
[hooks]
postinstall = "hk install --mise"
```

This installs or updates hooks when mise installs tools. If hk hooks are already installed globally, hk skips the local installation and cleans up stale local hooks.

## Call a mise task from a step

Use a task when a check is also useful outside Git hooks:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test"] {
        check = "mise run test"
      }
    }
  }
}
```

A step without a `glob` runs regardless of file matches. Add a pattern if the task should only run for selected file types.

If a task writes files outside the step’s selected paths, declare suitable dependencies or use `exclusive = true` to keep it from interfering with other steps.

## Share environment variables

Use mise’s `[env]` section for variables that should apply to project commands:

```toml
[env]
NODE_ENV = "development"
```

Use hk’s global, hook, or step `env` blocks for variables specific to linter commands. See [mise environments](https://mise.jdx.dev/environments/) and [hk configuration](/configuration).

## Run in CI

Once mise is available:

```sh
mise install
mise exec -- hk check --all
```

Install language package dependencies too, if the steps use them. See [continuous integration](/ci) for branch comparisons, profiles, and diagnostics.

## Per-directory environments (monorepos)

When `HK_MISE=1` is set, hk resolves the mise environment for each step's `dir` by
running `mise env` in that directory (cached, once per directory per run). Tools and
env vars defined by a subdirectory's mise config — such as a
[mise monorepo](https://mise.jdx.dev/tasks/monorepo.html) config root's `mise.toml` —
are available to steps running in that directory, even when hk is started from the
repo root:

```pkl
hooks {
    ["check"] {
        steps {
            ["oxlint"] = (Builtins.ox_lint) {
                // with HK_MISE=1, tools from subproject/mise.toml are on PATH
                dir = "subproject"
            }
        }
    }
}
```

Explicit step `env` values always win over the mise-provided environment.
