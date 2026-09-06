---
description: Learn the Pkl syntax needed to configure hk, reuse steps, and diagnose evaluation errors.
---

# Pkl essentials

hk uses [Pkl](https://pkl-lang.org/) for typed configuration. Most projects need only a few features: amend the schema, import builtins, define steps, and reuse them across hooks.

Pkl evaluates configuration. hk then runs the commands that configuration defines.

## Start with the schema

Every project configuration should amend hk’s base schema:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Builtins.pkl"
```

`amends` supplies the allowed properties and classes, such as `Step`, `Hook`, and `Group`. `import` makes another module available under its name, here `Builtins`.

Keep both package URLs on the same version. Changing the hk executable does not rewrite pinned imports in `hk.pkl`.

## Values and local variables

```pkl
local label = "lint"
local workers = 4
local enabled = true
local extensions = List("*.js", "*.ts")
```

Use `local` for helper values that are not part of hk’s schema. Without it, Pkl treats the value as a configuration property.

Strings use double quotes, booleans use `true` and `false`, and a list uses `List(...)`.

## Define a step

```pkl
local eslint = new Step {
  glob = List("*.js", "*.ts")
  check = "eslint {{files}}"
  fix = "eslint --fix {{files}}"
}
```

`new Step` creates an instance of the schema’s step class. `{{files}}` is an hk command template, expanded later when the step runs; it is not Pkl interpolation.

## Reuse steps in mappings

Hooks and steps are mappings keyed by name:

```pkl
local linters = new Mapping<String, Step> {
  ["eslint"] = Builtins.eslint
  ["prettier"] = Builtins.prettier
}

hooks {
  ["check"] { steps = linters }
  ["fix"] {
    fix = true
    steps = linters
  }
}
```

A mapping entry uses `["name"] = value`. Each name must be unique within the mapping.

You can add entries to a new mapping with a spread:

```pkl
local extended = new Mapping<String, Step> {
  ...linters
  ["shellcheck"] = Builtins.shellcheck
}
```

## Amend a builtin

Parentheses followed by an object body create a modified copy:

```pkl
local linters = new Mapping<String, Step> {
  ["prettier"] = (Builtins.prettier) {
    glob = List("*.js", "*.ts")
    exclude = List("**/generated/**")
  }
}
```

Unspecified properties keep the builtin’s values. Assigning a new list replaces that property’s list; it does not automatically append to it.

## Use raw strings for commands

Raw strings help when a command contains quotes or backslashes:

```pkl
local json_check = new Step {
  glob = "*.json"
  check = #"jq -e '.' {{files}} >/dev/null"#
}
```

For longer commands, use a multiline raw string:

```pkl
local test = new Step {
  check = #"""
    echo "Running tests"
    mise run test
    """#
}
```

The closing delimiter determines indentation. Keep the body indented consistently.

## Comments

```pkl
// A comment
/* A multiline comment */
/// A documentation comment
local explanation = "Documentation comments describe the following declaration."
```

## Share configuration across files

```pkl
amends "./hk.pkl"

hooks {
  ["check"] {
    steps {
      ["local-check"] {
        check = "make local-check"
      }
    }
  }
}
```

This is a local amendment of an existing project configuration. Save it as `hk.local.pkl` and keep it out of version control. The selected file amends `hk.pkl`; hk does not independently merge those two project files. See [local overrides](/configuration#hk-local-pkl).

## Validate and inspect

```sh
hk validate
hk check --plan
```

Validation evaluates the configuration without executing linter commands. A plan then shows how hk selects steps and files.

If the Pkl CLI is installed, inspect the evaluated module with:

```sh
pkl eval --format json hk.pkl
```

Use the [Pkl language reference](https://pkl-lang.org/main/current/language-reference/index.html) for features beyond these examples.

## Evaluators

hk includes [pklr](https://github.com/jdx/pklr) and uses it by default. If a configuration uses a Pkl feature that pklr does not yet support, try the Pkl CLI backend:

```sh
mise use -g pkl
HK_PKL_BACKEND=pkl hk validate
```

Set `HK_PKL_BACKEND=pkl` in the environment used by Git as well if your hooks need that backend.

## Caching

The built-in evaluator persists downloaded packages and seeds the cache with the Pkl package matching the running hk version. Use [`HK_PKL_OFFLINE`](/environment_variables#hk-pkl-offline) to require cached or embedded packages without network access.

Release builds cache evaluated configuration; debug builds disable this cache by default. When diagnosing an unexpected result after changing an import or evaluation input, bypass or clear the cache:

```sh
HK_CACHE=0 hk validate
hk cache clear
```

Use hk’s runtime settings, profiles, and command environment where possible instead of making configuration depend on changing evaluation inputs.
