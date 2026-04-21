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

## Project Setup

Use [`hk init`](/cli/init) to generate a `hk.pkl` file:

```sh
hk init
```

## Global Configuration

You can create a global configuration file that will be applied to all projects. This is useful for setting up consistent linting rules across multiple repositories. By default, hk will look for this file in your home directory.

The global configuration file follows the same format as `hk.pkl` and can be used to define global hooks and linters. Project-specific settings in `hk.pkl` can override or extend the global configuration.

## `hk.pkl`

This will generate a `hk.pkl` file in the root of the repository, here's an example `hk.pkl` with eslint and prettier linters:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.43.0/hk@1.43.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.43.0/hk@1.43.0#/Builtins.pkl"

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

## Usage

Inside a git repository with a `hk.pkl` file, run [`hk install`](/cli/install) to configure git to use the hooks defined in `hk.pkl`:

```sh
hk install
```

This will install the hooks for the repository like `pre-commit` and `pre-push` if they are defined in `hk.pkl`. Running `git commit` would now run the linters defined above in our example through the pre-commit hook.

On **Git 2.54 or newer**, `hk install` writes [config-based hooks](https://github.blog/open-source/git/highlights-from-git-2-54/) (`git config hook.hk-<event>.command`) instead of script files in `.git/hooks/`. This keeps the hooks directory untouched and composes cleanly with other hook managers. On older Git it falls back to writing script shims — no configuration needed, hk detects the installed git version automatically. Pass `--legacy` to force the shim mode.

## Install Hooks Globally (Git 2.54+)

With Git 2.54+, you can install hk hooks once in your **user-wide** `~/.gitconfig` and they apply to every repository on your machine:

```sh
hk install --global
```

This writes `hook.hk-<event>.command` entries to your global git config for the common client-side hooks (`pre-commit`, `pre-push`, `commit-msg`, `prepare-commit-msg`, `post-checkout`, `post-merge`, `post-rewrite`, `pre-rebase`, `post-commit`). Each invocation is a **silent no-op in repos that don't have an `hk.pkl`**, so you can safely enable it everywhere without breaking unrelated projects.

To remove the global install:

```sh
hk uninstall --global
```

Per-repository `hk install` still works alongside `--global` — the local entries simply replace the global defaults for that repo.

### Configuring manually in `~/.gitconfig`

If you'd rather set this up by hand, add a block like the following to your `~/.gitconfig`:

```ini
[hook "hk-pre-commit"]
    command = hk run pre-commit --from-hook "$@"
    event = pre-commit
[hook "hk-pre-push"]
    command = hk run pre-push --from-hook "$@"
    event = pre-push
[hook "hk-commit-msg"]
    command = hk run commit-msg --from-hook "$@"
    event = commit-msg
```

The `--from-hook` flag tells hk to exit silently when the project has no `hk.pkl` or doesn't define that event. Use `mise x -- hk` instead of `hk` in the `command` if you manage hk via mise and don't auto-activate it.

To disable hk for a single repo without uninstalling globally, set `hook.hk-<event>.enabled = false` in that repo's `.git/config`.

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
