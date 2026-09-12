---
description: Install hk, configure your first checks, and run the same steps in Git hooks and CI.
---

# Getting started

Set up hk in an existing Git repository, then use the same linters when you commit, work locally, and run CI.

## Installation

Choose one installation method:

::: code-group

```sh [mise]
mise use hk
```

```sh [Homebrew]
brew install hk
```

```sh [Cargo]
cargo install hk --locked
```

:::

Verify the installation:

```sh
hk --version
```

Prebuilt binaries are also available from [GitHub releases](https://github.com/jdx/hk/releases). hk uses the built-in [pklr evaluator](/pkl_introduction#evaluators) by default, so you do not need to install the Pkl CLI.

## Project setup

From the root of your repository, generate a configuration:

```sh
hk init
```

hk detects tools from project files and creates `hk.pkl`. Review its steps before running them. To select tools and hooks yourself, use `hk init --interactive`.

::: tip Make the linters available
Builtins configure commands; they do not install the tools they invoke. Install the selected linters with your project’s package manager or [mise](/mise_integration), and make sure hk can find them on `PATH`.
:::

## Install hooks

Choose the scope that fits your setup:

| Scope                       | Command               | Behavior                                                                               |
| --------------------------- | --------------------- | -------------------------------------------------------------------------------------- |
| All repositories, Git 2.54+ | `hk install --global` | Install once in your user Git config; projects without an hk configuration are skipped |
| Current repository          | `hk install`          | Install the hooks defined in this project; supports older Git versions                 |

On Git 2.54+, hk uses Git’s configuration-based hooks. On older Git, a per-repository install writes script shims. Use `hk install --legacy` to request shims explicitly.

If hk is already installed globally, `hk install` skips the local installation and cleans up stale local hk hooks. `--force-local` overrides that behavior, but combining local and global hooks can cause duplicate runs.

::: tip Using mise tools in Git hooks
Use `hk install --mise` or `hk install --global --mise` to launch hooks through `mise x`. This makes project tools available even when Git runs outside an activated shell, such as from an editor. mise itself must be on Git’s `PATH`.
:::

Commit `hk.pkl` so your team can share the configuration. Hook installation is local to each developer’s machine or clone.

To remove an installation, use `hk uninstall` or `hk uninstall --global`. See the [install reference](/cli/install) for all options.

## Your first configuration

This complete example runs Prettier, ESLint, and Ruff. Install and configure those tools first, or replace them with [builtins](/builtins) that match your project.

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Builtins.pkl"

local linters = new Mapping<String, Step> {
  ["prettier"] = Builtins.prettier
  ["eslint"] = Builtins.eslint
  ["ruff"] = Builtins.ruff
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = linters
  }
  ["check"] { steps = linters }
  ["fix"] {
    fix = true
    steps = linters
  }
}
```

The `amends` line loads hk’s configuration schema. `Builtins` supplies reusable step definitions. The local `linters` mapping lets all three hooks use the same steps.

In this configuration, `pre-commit` fixes staged files while unstaged work is stashed. `check` checks your working tree, and `fix` applies fixes to it. Steps whose file patterns do not match any selected files are skipped.

Validate the configuration without running its linters:

```sh
hk validate
```

## Checking and fixing code

```sh
hk check             # Check modified files
hk fix               # Apply available fixes
hk check --all       # Check all files, useful for CI
hk check src/main.ts # Check a specific file
hk check --step eslint
```

With the configuration above, modified files include staged, unstaged, and untracked files. `--all` selects tracked files plus eligible untracked files; ignore rules and exclusions still apply. Hook settings and flags can change file selection.

Check commands should be read-only. Fix commands may edit files, and some findings need a manual fix. Fixes may be staged automatically. Review `git diff` and `git diff --cached`, or use `hk fix --no-stage` to leave fixes unstaged.

## Preview a run

Use the plan to see which steps and files hk selects:

```sh
hk check --plan
hk check --why eslint
hk check --all --plan --json
```

These commands do not execute the hook’s steps. See [troubleshooting](/logging) if a step is missing or behaves unexpectedly.

## Running hooks

After installation, Git invokes configured hooks automatically. You can also invoke them directly:

```sh
hk run pre-commit
```

A manual hook run uses the hook’s configured behavior, including fixes, staging, and stashing. To inspect it first, use `hk run pre-commit --plan`.

## Next steps

- [Git hooks and stashing](/hooks): control automatic fixes and partial commits.
- [Continuous integration](/ci): check a full repository or a branch.
- [Configuration examples](/reference/examples/): start from a JavaScript, Python, or monorepo setup.
- [Configuration](/configuration): customize steps, profiles, and local overrides.
