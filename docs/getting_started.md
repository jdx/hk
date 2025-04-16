# Getting Started

A tool for running hooks on files in a git repository.

::: warning
hk is in an early beta stage. You may encounter bugs and breaking changes until 1.x.
:::

## Installation

Use [mise-en-place](https://github.com/jdx/mise) to install hk (you'll also need the `pkl` cli):

```sh
mise use hk pkl
hk --version
```

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

Use `hk generate` to generate a `hk.pkl` file:

```sh
hk generate
```

## `hk.pkl`

This will generate a `hk.pkl` file in the root of the repository, here's an example `hk.pkl` with eslint and prettier linters:

```pkl
amends "package://github.com/jdx/hk/releases/download/v0.7.5/hk@0.7.5#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v0.7.5/hk@0.7.5#/Builtins.pkl"

local linters {
    // linters can be manually defined
    ["eslint"] {
        // the files to run the linter on, if no files are matched, the linter will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = List("*.js"; "*.ts")
        // a command that returns non-zero to fail the check
        check = "eslint {{files}}"
    }
    // linters can also be specified with the builtins pkl library
    ["prettier"] = Builtins.prettier
    // with pkl, builtins can also be extended:
    ["prettier-yaml"] = (Builtins.prettier) {
        glob = List("*.yaml"; "*.yml")
    }
}

hooks {
    ["pre-commit"] {
        fix = true           // runs the "fix" step of linters to modify files
        stash = "patch-file" // stashes unstaged changes when running fix steps
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

Inside a git repository with a `hk.pkl` file, run:

```sh
hk install
```

This will install the hooks for the repository like `pre-commit` and `pre-push` if they are defined in `hk.pkl`. Running `git commit` would now run the linters defined above in our example through the pre-commit hook.

### `core.hooksPath`

As an alternative to using `hk install`, you can run `git config --local core.hooksPath .hooks` to use the hooks defined in the `.hooks` directory of the repository:

```sh
#!/bin/sh
hk run pre-commit
```

## Running Hooks

To explicitly run the hooks without going through git, use the `hk run` command.

```sh
hk run pre-commit
```

This will run the `pre-commit` hook for the repository. This will run against all files that are staged for commit. To run against all files in the repository, use the `--all` flag.

```sh
hk run pre-commit --all
```

To run a specific linter, use the `--linter` flag.

```sh
hk run pre-commit --linter eslint
```
