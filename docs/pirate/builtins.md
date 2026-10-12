---
outline: [2, 3]
description: Meet the standing crew who come with the ship. Browse reusable linter and formatter definitions, refit them to yer needs, and learn which tools ye must bring aboard yerself.
sourceHash: 3176496cdc02
---

# The standing crew: built-in linters

Builtins be the standing crew who come with the ship: reusable Pkl step definitions for linters, formatters, and hk's own utilities. Between them they bring file patterns, check and fix commands, and optimizations such as diff output.

**Bring the tools aboard separately.** A builtin calls on executables from yer environment; it does not install them. Provision them with yer project's package manager or with [mise, the quartermaster](/mise_integration).

## Muster a builtin {#use-a-builtin}

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.6.0/hk@2.6.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.6.0/hk@2.6.0#/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["prettier"] = Builtins.prettier
      ["eslint"] = Builtins.eslint
    }
  }
}
```

`Builtins.prettier` is the Pkl property name, the name the hand goes by on the roster. The step name, `"prettier"`, is yer own label for that hand, the one ye name in calls such as `hk check --step prettier`.

Aye, and keep the schema and Builtins imports on the same version. The roster below describes the version of the source this website was built from; an older pinned package may differ.

## Refit a builtin {#customize-a-builtin}

Amend a builtin to keep its defaults while ye change particular properties:

```pkl
["prettier"] = (Builtins.prettier) {
  glob = List("*.js", "*.ts", "*.json")
  exclude = List("**/generated/**")
  batch = false
}
```

Mind this: a property assignment replaces that property whole. If ye override `glob` or `exclude`, include every pattern ye want to keep.

Use dependencies to set the order the hands work in, and profiles (the ship's watches) for optional checks:

```pkl
["prettier"] = (Builtins.prettier) {
  depends = "eslint"
}
["mypy"] = (Builtins.mypy) {
  profiles = List("types")
}
```

Run the type checker by calling its watch: `hk check --profile types`. See the [configuration charts](/configuration) for groups, workspaces, and command templates.

## Utilities hk carries aboard {#utilities-included-with-hk}

Hands such as `Builtins.trailing_whitespace`, `Builtins.newlines`, and `Builtins.check_merge_conflict` call on [`hk util`](/cli/util) commands. These sail with hk itself and need no separate linter executable.

## Tools on hand {#tool-availability}

A builtin charts how hk calls a tool; it does not install that tool. The
executable the builtin uses must be on `PATH`, or the step must use a `prefix`
that resolves it.

With the standing order [`HK_MISE=1`](/mise_integration#per-directory-environments-monorepos),
hk resolves the quartermaster's mise environment for the step's own directory, so
tools declared in a subproject's `mise.toml` are on hand without a prefix. For a
Node tool installed locally by aube, prefix its builtin with `aube exec`:

```pkl
["eslint"] = (Builtins.eslint) {
  prefix = List("aube", "exec")
}
```

Mind ye, use an argv list for builtins backed by structured commands. A string prefix such
as `"aube exec"` or `"mise x --"` cannot be combined with those commands.

## The crew roster {#available-builtins}

The roster below is generated from the builtin definitions themselves, so it stays in plain English. Each entry gives the exact Pkl property to call the hand by. Look to the [source definitions](https://github.com/jdx/hk/tree/main/pkl/builtins) for more options and tests.

<!--@include: ../gen/builtins.md-->

## Bring a tool of yer own aboard {#add-a-tool-of-your-own}

If there's no builtin for yer tool, define a step with `glob`, `check`, and an optional `fix` command. Only enable batching if the tool can process independent subsets of the files correctly.

See [custom steps](/reference/examples/custom-linters) for a complete example. And, as the song goes, all you hands who would sign aboard: see [contributing](/contributing#add-a-builtin) to add a reusable definition to the standing crew.
