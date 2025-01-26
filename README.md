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

Use `hk generate` to generate a `hk.toml` file:

```
hk generate
```

This will generate a `hk.toml` file in the root of the repository like this:

```
[[pre-commit]]
plugin = "end-of-file-fixer"
```

## Usage

Inside a git repository with a `hk.toml` file, run:

```
hk install
```

This will install the hooks for the repository like `pre-commit` and `pre-push` if they are defined in the `hk.toml` file.

## Running Hooks

To explicitly run the hooks without committing, use the `hk run` command.

```
hk run pre-commit
```

This will run the `pre-commit` hooks for the repository. This will run against all files that are staged for commit. To run against all files in the repository, use the `--all` flag.

```
hk run pre-commit --all
```

To run a specific hook, use the `--hook` flag.

```
hk run pre-commit --hook end-of-file-fixer
```
