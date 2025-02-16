# hk

A tool for running hooks on files in a git repository.

> [!CAUTION]
> This is a work in progress.

## Installation

Use [mise-en-place](https://github.com/jdx/mise) to install hk:

```
mise use hk
hk --version
```

## Project Setup

Use `hk generate` to generate a `hk.pkl` file:

```
hk generate
```

## `hk.pkl`

This will generate a `hk.pkl` file in the root of the repository, here's an example `hk.pkl` with eslint and prettier hooks:

```pkl
amends "package://hk.jdx.dev/hk@0.1.0#/hk.pkl"
amends "package://hk.jdx.dev/hk@0.1.0#/builtins.pkl"

pre_commit {
    // hooks can be manually defined
    ["eslint"] {
        // the files to run the hook on, if no files are matched, the hook will be skipped
        // this will filter the staged files and return the subset matching these globs
        glob = new { "*.js"; "*.ts" }
        // the command to run the hook on the files that makes no changes
        run = "eslint {{files}}"
        // the command to run the hook on the files that fixes them (used by default)
        fix = "eslint --fix {{files}}"
    }
    // hooks can also be specified with the builtins pkl library
    ["prettier"] = new builtins.Prettier {}
}
```

## Usage

Inside a git repository with a `hk.pkl` file, run:

```
hk install
```

This will install the hooks for the repository like `pre-commit` and `pre-push` if they are defined in `hk.pkl`. Running `git commit` would now run the pre_commit hooks defined above in our example.

## Running Hooks

To explicitly run the hooks without going through git, use the `hk run` command.

```
hk run pre-commit
```

This will run the `pre-commit` hooks for the repository. This will run against all files that are staged for commit. To run against all files in the repository, use the `--all` flag.

```
hk run pre-commit --all
```

To run a specific step, use the `--step` flag.

```
hk run pre-commit --step eslint
```
