---
outline: [2, 3]
description: Browse reusable linter and formatter definitions, customize them, and understand tool requirements.
---

# Built-in linters

Builtins are reusable Pkl step definitions for linters, formatters, and hk’s own utilities. They supply file patterns, check and fix commands, and optimizations such as diff output.

**Install the tools separately.** A builtin invokes executables from your environment; it does not install them. Use your project’s package manager or [mise](/mise_integration).

## Use a builtin

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.59.0/hk@1.59.0#/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["prettier"] = Builtins.prettier
      ["eslint"] = Builtins.eslint
    }
  }
}
```

`Builtins.prettier` is the Pkl property name. The step name, `"prettier"`, is your label for selecting the step in commands such as `hk check --step prettier`.

Keep the schema and Builtins imports on the same version. The catalogue below describes the version of the source used to build this website; an older pinned package may differ.

## Customize a builtin

Amend a builtin to keep its defaults while changing specific properties:

```pkl
["prettier"] = (Builtins.prettier) {
  glob = List("*.js", "*.ts", "*.json")
  exclude = List("**/generated/**")
  batch = false
}
```

A property assignment replaces that property. If you override `glob` or `exclude`, include every pattern you want to retain.

Use dependencies for ordering and profiles for optional checks:

```pkl
["prettier"] = (Builtins.prettier) {
  depends = "eslint"
}
["mypy"] = (Builtins.mypy) {
  profiles = List("types")
}
```

Run the type checker with `hk check --profile types`. See [configuration](/configuration) for groups, workspaces, and command templates.

## Utilities included with hk

Steps such as `Builtins.trailing_whitespace`, `Builtins.newlines`, and `Builtins.check_merge_conflict` invoke [`hk util`](/cli/util) commands. These need no separate linter executable.

## Tool availability

Builtins configure how hk invokes a tool; they do not install that tool. The
executable used by the builtin must be on `PATH`, or the step must use a `prefix`
that resolves it.

With [`HK_MISE=1`](/mise_integration#per-directory-environments-monorepos), hk
resolves the mise environment for the step's directory, so tools declared in a
subproject's `mise.toml` are available without a prefix. For a Node tool installed
locally by aube, prefix its builtin with `aube exec`:

```pkl
["eslint"] = (Builtins.eslint) {
  prefix = List("aube", "exec")
}
```

Use an argv list for builtins backed by structured commands. A string prefix such
as `"aube exec"` or `"mise x --"` cannot be combined with those commands.

## Available builtins

The following catalogue is generated from the builtin definitions. Each entry includes the exact Pkl property to use. Refer to the [source definitions](https://github.com/jdx/hk/tree/main/pkl/builtins) for additional options and tests.

<!--@include: ./gen/builtins.md-->

## Add a tool of your own

If there is no builtin, define a step with `glob`, `check`, and an optional `fix` command. Only enable batching if the tool can process independent subsets of files correctly.

See [custom steps](/reference/examples/custom-linters) for a complete example, or [contributing](/contributing#add-a-builtin) to contribute a reusable definition.
