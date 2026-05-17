# Getting Started

A tool for running hooks on files in a git repository.

## Installation

Use [mise-en-place](https://github.com/jdx/mise) to install hk:

```sh
mise use hk
hk --version
```

:::tip
By default hk uses the pkl CLI to evaluate configuration. Set `HK_PKL_BACKEND=pklr` to use the built-in Rust evaluator instead, which removes the pkl CLI dependency entirely. This is experimental — see [pkl introduction](/pkl_introduction) for details.
:::

:::tip
mise-en-place integrates well with hk. Features common in similar git-hook managers like dependency management, task dependencies, and env vars can be provided by mise.

See [mise integration](/mise_integration) for more information.
:::

Or install from source with `cargo`:

```sh
cargo install hk
```

Other installation methods:

- [`brew install hk`](https://formulae.brew.sh/formula/hk)
- [`aqua g -i jdx/hk`](https://github.com/aquaproj/aqua-registry/blob/main/pkgs/jdx/hk/registry.yaml)

## Install Hooks (recommended: global)

On **Git 2.54+**, the recommended way to set up hk is to install hooks **once, globally** into your `~/.gitconfig`. They then apply to every repository on your machine — and are a **silent no-op in any repo that doesn't have an `hk.pkl`**, so it's safe to enable everywhere:

```sh
hk install --global
```

This writes `hook.hk-<event>.command` entries to your global git config for the common client-side hooks (`pre-commit`, `pre-push`, `commit-msg`, `prepare-commit-msg`, `post-checkout`, `post-merge`, `post-rewrite`, `pre-rebase`, `post-commit`). After this, adding an `hk.pkl` to a project is all you need — no per-repo install step.

To remove the global install:

```sh
hk uninstall --global
```

:::tip
Prefer this to per-repo `hk install`: no need to re-run `hk install` in each clone, new repos just work, and projects without an `hk.pkl` are unaffected.
:::

### Per-repository install (alternative)

If you can't use Git 2.54+, or you want hk to apply only to specific repos, use per-repo install from inside a project that has an `hk.pkl`:

```sh
hk install
```

This installs only the hooks defined in the project's `hk.pkl`. On Git 2.54+ it writes config-based hooks (`git config hook.hk-<event>.command`); on older Git it falls back to [script shims](https://github.blog/open-source/git/highlights-from-git-2-54/) in `.git/hooks/`. Pass `--legacy` to force shim mode.

:::warning
Running per-repo `hk install` on top of `hk install --global` causes hk to fire **twice per event** — Git aggregates `hook.<name>.command` entries across every scope. If you want only the local install in a repo that already has the global install active, disable the global entries in that repo with `git config --local hook.hk-<event>.enabled false`.
:::

### Configuring manually in `~/.gitconfig`

If you'd rather set this up by hand instead of running `hk install --global`, add a block like the following to your `~/.gitconfig`:

```ini
[hook "hk-pre-commit"]
    command = test "${HK:-1}" = "0" || hk run pre-commit --from-hook "$@"
    event = pre-commit
[hook "hk-pre-push"]
    command = test "${HK:-1}" = "0" || hk run pre-push --from-hook "$@"
    event = pre-push
[hook "hk-commit-msg"]
    command = test "${HK:-1}" = "0" || hk run commit-msg --from-hook "$@"
    event = commit-msg
```

The `--from-hook` flag tells hk to exit silently when the project has no `hk.pkl` or doesn't define that event. The `test "${HK:-1}" = "0" ||` prefix is an escape hatch: run `HK=0 git commit` to bypass hooks for a single command. Use `mise x -- hk` instead of `hk` in the `command` if you manage hk via mise and don't auto-activate it.

To disable hk for a single repo without uninstalling globally, set `hook.hk-<event>.enabled = false` in that repo's `.git/config`.

## Project Setup

With hooks installed globally, enabling hk for a project is just:

```sh
hk init
```

This generates an `hk.pkl` file in the root of the repository. `git commit` will now run the linters defined in that file via the already-installed global `pre-commit` hook — no per-repo `hk install` needed.

## Global `hkrc` Configuration

Separately from global *hooks*, you can also create a global *config* file that is merged into every project's `hk.pkl`. This is useful for setting up consistent linting rules across multiple repositories. By default, hk looks for this file at `~/.config/hk/config.pkl`. See [hkrc](/configuration#hkrc) for details.

## `hk.pkl`

This will generate a `hk.pkl` file in the root of the repository, here's an example `hk.pkl` with eslint and prettier linters:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.46.0/hk@1.46.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.46.0/hk@1.46.0#/Builtins.pkl"

local linters = new Mapping<String, Step> {
    // linters can be manually defined
    ["eslint"] {
        // the files to run the linter on, if no files are matched, the linter will be skipped
        glob = List("*.js", "*.ts")
        // a command that returns non-zero to fail the check
        check = "eslint {{files}}"
    }
    // linters can also be specified with the builtins pkl library
    ["prettier"] = Builtins.prettier
    // with pkl, builtins can also be extended:
    ["prettier-yaml"] = (Builtins.prettier) {
        glob = List("*.yaml", "*.yml")
    }
}

hooks {
    ["pre-commit"] {
        fix = true    // runs the "fix" step of linters to modify files
        stash = "git" // stashes unstaged changes when running fix steps
        steps {
            ["prelint"] {
                check = "mise run prelint"
                exclusive = true // blocks other steps from starting until this one finishes
            }
            ...linters
            ["postlint"] {
                check = "mise run postlint"
                exclusive = true
            }
        }
    }
}
```

See [configuration](/configuration) for more information on the `hk.pkl` file.

## Checking and Fixing Code

You can check or fix code with [`hk check`](/cli/check) or [`hk fix`](/cli/fix)—by convention, "check" means files should not be modified and "fix"
should verify everything "check" does but also modify files to fix any issues. By default, `hk check|fix` run against any modified files in the repo.

:::tip
Use `hk check --all` in CI to lint all the files in the repo or `hk check --from-ref main` to lint files that have changed since the `main` branch.
:::

## Running Hooks

To explicitly run a hook without going through git, use the [`hk run`](/cli/run) command. This is generally useful for testing hooks locally.

```sh
hk run pre-commit
```
