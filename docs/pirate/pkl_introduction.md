---
description: Learn the Pkl ye need to draw hk's charts, muster the same hands across hooks, and find out why the charts won't evaluate.
sourceHash: 8747ce496cf7
---

# Pkl essentials for the chart room

hk draws its charts in [Pkl](https://pkl-lang.org/), the chart-maker's language, so they come out typed and true. Most ships need only a handful of its features: amend the schema, import the builtins, define steps, and reuse them across hooks.

Pkl evaluates the charts. Then hk runs the commands those charts set down.

## Start from the schema, the base chart {#start-with-the-schema}

Every ship's configuration should amend hk's base schema, the base chart every set of charts is drawn over:

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Builtins.pkl"
```

`amends` supplies the properties and classes ye're allowed, such as `Step`, `Hook`, and `Group`. `import` brings another module aboard under its own name, here `Builtins`.

Keep both package URLs on the same version. Changing the hk executable does not rewrite the pinned imports in `hk.pkl`; they stay where ye pinned them.

## Values and local variables {#values-and-local-variables}

```pkl
local label = "lint"
local workers = 4
local enabled = true
local extensions = List("*.js", "*.ts")
```

Use `local` for helper values that aren't part of hk's schema: working figures for yer own reckoning rather than settings. Leave it off, and Pkl takes the value for a configuration property.

Strings sail in double quotes, booleans are `true` and `false`, and a list is `List(...)`.

## Sign on a hand: define a step {#define-a-step}

```pkl
local eslint = new Step {
  glob = List("*.js", "*.ts")
  check = "eslint {{files}}"
  fix = "eslint --fix {{files}}"
}
```

`new Step` creates an instance of the schema's step class: a fresh hand, signed on under the schema's rules. `{{files}}` is an hk command template, filled in later, when the step runs; it is not Pkl interpolation.

## Muster the same hands in mappings {#reuse-steps-in-mappings}

Hooks and steps are mappings keyed by name, like a muster roll. Prefer top-level `steps` for lookouts shared by `check`, `fix`, and `pre-commit`:

```pkl
steps {
  ["eslint"] = Builtins.eslint
  ["prettier"] = Builtins.prettier
}
```

A mapping entry is written `["name"] = value`. Each name must be unique within its mapping: no two hands answer to the same name on one roll.

Top-level `steps` is optional. To share one mapping among only the hooks ye choose and spell out yerself, use a local helper instead:

```pkl
local linters = new Mapping<String, Step> {
  ["eslint"] = Builtins.eslint
  ["prettier"] = Builtins.prettier
}

hooks {
  ["check"] { steps = linters }
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ...linters
      ["shellcheck"] = Builtins.shellcheck
    }
  }
}
```

## Refit a builtin: amend it {#amend-a-builtin}

Parentheses followed by an object body make a modified copy, refitted to yer liking:

```pkl
steps {
  ["prettier"] = (Builtins.prettier) {
    glob = List("*.js", "*.ts")
    exclude = List("**/generated/**")
  }
}
```

Any property ye leave unspecified keeps the builtin's value. Assigning a new list replaces that property's list outright; it does not append to it for ye.

## Use raw strings for knotty commands {#use-raw-strings-for-commands}

Raw strings help when a command is tangled with quotes or backslashes:

```pkl
local json_check = new Step {
  glob = "*.json"
  check = #"jq -e '.' {{files}} >/dev/null"#
}
```

For a longer command, spin out a multiline raw string:

```pkl
local test = new Step {
  check = #"""
    echo "Running tests"
    mise run test
    """#
}
```

The closing delimiter decides the indentation. Keep the body indented consistently, matey.

## Comments in the margin {#comments}

Scrawl yer notes on the charts like so:

```pkl
// A comment
/* A multiline comment */
/// A documentation comment
local explanation = "Documentation comments describe the following declaration."
```

## Share the charts across files {#share-configuration-across-files}

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

This is a local amendment of an existing project configuration: yer own marks on the ship's charts. Save it as `hk.local.pkl` and keep it out of version control. The file hk selects amends `hk.pkl`; hk itself does not merge those two project files. See [local overrides, yer own marks on the charts](/configuration#hk-local-pkl).

## Import a whole haul o' files at once {#import-many-files-at-once}

`import*` is a glob import: it hauls in and binds every file matching the pattern, each keyed by
its path relative to the module doing the importing.

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"

import* "generated/*.pkl" as generated

hooks {
  ["check"] {
    steps = new Mapping<String, Step> {
      for (_, mod in generated) {
        ...mod.STEPS
      }
    }
  }
}
```

Each `generated/*.pkl` file brings its own `STEPS` aboard, so a build script can
add or remove step definitions, signing hands on and off, without editing `hk.pkl`:

```pkl
// generated/prettier.pkl
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"

STEPS: Mapping<String, Config.Step> = new {
  ["prettier"] {
    glob = List("*.md")
    check = "prettier --check {{files}}"
  }
}
```

Letting each file name its own hands keeps the keys independent of the file
names. An imported module is not itself a `Step`, so read a typed property such
as `STEPS` back out of it rather than assigning the module into `steps` directly.

`import*` also works as an expression, which binds the mapping to a local
instead of a module-level name:

```pkl
local generated = import*("generated/*.pkl")
```

A pattern that matches nothing brings back an empty mapping, and the file holding the
import is skipped when the pattern would match it.

The pattern is resolved against the filesystem on every run: add, remove or
rename a file the pattern matches, and it takes effect on the next hk command,
with no need to touch `hk.pkl` or run `hk cache clear`.

## Validate and inspect the charts {#validate-and-inspect}

```sh
hk validate
hk check --plan
```

Validation evaluates the charts without running a single linter command. A passage plan then shows how hk selects its hands and its cargo: the steps and the files.

Validation fails on charts hk cannot sail, such as a dependency cycle between steps or a glob that does not compile. It also hollers warnings, without failing, for settings that load but likely aren't what ye meant: a misspelled property (release builds otherwise drop it silently), a `depends` entry that names an unknown step, a group, or a step in a later group, a step with no command, and a glob that starts with `./`, `/` or `!` or lists patterns with commas.

If ye have the Pkl CLI on hand, look over the evaluated module with:

```sh
pkl eval --format json hk.pkl
```

For features beyond these examples, steer by the [Pkl language reference](https://pkl-lang.org/main/current/language-reference/index.html).

## Evaluators aboard {#evaluators}

hk sails with [pklr](https://github.com/jdx/pklr) built in, and always uses it to evaluate project, local, and global configuration. The standalone Pkl CLI is still handy for inspecting modules, but hk does not need it to run, and never falls back to it as an evaluator.

## Caching the charts {#caching}

The built-in evaluator lays up the packages it downloads in its locker for later runs, and stocks that locker from the start with the Pkl package matching the running hk version. Use [`HK_PKL_OFFLINE`](/environment_variables#hk-pkl-offline) to require cached or embedded packages, with no network access: no boats go ashore.

Release builds cache the evaluated configuration; debug builds turn this cache off by default. The values of environment variables (the standing orders) that the configuration reads with `read("env:NAME")` or `read?("env:NAME")` are part of the cache key. hk re-evaluates the configuration only when no cache entry exists for the current values; set a variable back to an earlier value, and hk reuses that entry. Files read as resources, such as `read("data.txt")`, are not tracked: change one, and the cache won't notice. When the charts give ye an unexpected result after ye change an import or an evaluation input, bypass the cache or clear it:

```sh
HK_CACHE=0 hk validate
hk cache clear
```

Where ye can, use hk's runtime settings, profiles, and command environment, rather than making the charts depend on evaluation inputs that shift like the tide.
