# hk

A fast, language-agnostic git hook manager and project linter.

hk runs linters concurrently while coordinating access to files with read/write locks. This lets
formatters and other tools safely work on overlapping files without racing or silently overwriting
one another's changes.

- Runs independent checks and fixes in parallel
- Safely handles partially staged files by stashing and restoring unstaged changes
- Includes [built-in configurations](https://hk.jdx.dev/builtins) for common linters and formatters
- Uses typed [Pkl](https://pkl-lang.org/) configuration
- Integrates with [mise](https://mise.jdx.dev/) for tool and task management
- Provides fast native checks for common issues such as trailing whitespace and merge conflicts

## Quick start

Install hk with mise:

```sh
mise use hk
hk --version
```

With Git 2.54 or newer, install hk's hooks once for every repository on your machine:

```sh
hk install --global
```

Then enable hk in a project:

```sh
cd my-project
hk init
```

`hk init` creates an `hk.pkl` configuration. Choose the linters you want, then commit as usual;
hk runs the configured `pre-commit` hook automatically. Repositories without an `hk.pkl` are left
untouched by the global hooks.

On older Git versions, run `hk install` in each project instead. See the
[getting started guide](https://hk.jdx.dev/getting_started) for Homebrew, Cargo, and Aqua installation
options and detailed hook setup.

## Example configuration

The generated `hk.pkl` uses hk's built-in linter definitions, which you can extend when a project
needs different behavior:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.56.1/hk@1.56.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.56.1/hk@1.56.1#/Builtins.pkl"

local linters = new Mapping<String, Step> {
  ["eslint"] = Builtins.eslint
  ["prettier"] = (Builtins.prettier) {
    glob = List("*.js", "*.ts", "*.json", "*.md")
  }
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = linters
  }
  ["check"] {
    steps = linters
  }
  ["fix"] {
    fix = true
    steps = linters
  }
}
```

Run the same checks directly at any time:

```sh
hk check        # check modified files
hk fix          # fix modified files
hk check --all  # check the entire repository, useful in CI
```

## Documentation

- [Getting started](https://hk.jdx.dev/getting_started)
- [Configuration reference](https://hk.jdx.dev/configuration)
- [Configuration examples](https://hk.jdx.dev/reference/examples/)
- [Built-in linters](https://hk.jdx.dev/builtins)
- [CLI reference](https://hk.jdx.dev/cli/)
- [Why hk?](https://hk.jdx.dev/why-hk)
- [Contributing](CONTRIBUTING.md)

## Sponsors

hk is sponsored by [entire.io](https://entire.io) and [37signals](https://37signals.com).

[View all sponsors](https://jdx.dev/sponsors.html).

## Demo

![hk demo](docs/public/hk-demo.gif)

## CI

<p>
  <a href="https://namespace.so">
    <img src="docs/public/namespace-logo.svg" alt="Namespace" width="64">
  </a>
</p>

Thanks to [Namespace](https://namespace.so) for providing CI for hk.
