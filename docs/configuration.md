---
outline: deep
description: Configure hooks, steps, file selection, profiles, local overrides, and runtime settings.
---

# Configuration

hk reads `hk.pkl` to decide which steps to run and how to run them. Start with a shared set of linters, then add file filters, dependencies, and profiles as your project needs them.

For a first setup, use [getting started](/getting_started). For complete configurations, see the [examples](/reference/examples/).

## `hk.pkl`

A configuration amends hk’s [Pkl schema](/pkl_introduction) and defines named hooks:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Builtins.pkl"

local linters = new Mapping<String, Step> {
  ["eslint"] = Builtins.eslint
  ["prettier"] = Builtins.prettier
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = linters
  }
  ["check"] { steps = linters }
  ["fix"] {
    fix = true
    steps = linters
  }
}
```

`pre-commit` applies fixes to staged files while unstaged work is saved. The `check` and `fix` hooks provide the local commands. They are hooks, not individual steps.

### Config file paths

Starting in the current directory, hk walks upward. At each directory it checks these paths in order, using the first match:

| Order | Path                   | Purpose                               |
| ----- | ---------------------- | ------------------------------------- |
| 1     | `hk.local.pkl`         | Local project override                |
| 2     | `.config/hk.local.pkl` | Local override under `.config/`       |
| 3     | `hk.pkl`               | Shared project configuration          |
| 4     | `.config/hk.pkl`       | Shared configuration under `.config/` |

[`HK_FILE`](/environment_variables#hk-file) selects a specific configuration instead. hk selects one project file; it does not merge every file it finds.

### `hk.local.pkl`

Use Pkl’s `amends` to extend the shared project configuration locally:

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

Add `hk.local.pkl` to `.git/info/exclude` or the project’s `.gitignore`. This example preserves inherited steps and adds one. Assign a new mapping when you want to replace the step list:

```pkl
amends "./hk.pkl"

hooks {
  ["check"] {
    steps = new Mapping<String, Step> {
      ["local-check"] { check = "make local-check" }
    }
  }
}
```

## Define a step

A step selects files and declares commands:

```pkl
local eslint = new Step {
  glob = List("*.js", "*.ts")
  exclude = List("**/generated/**")
  check = "eslint {{files}}"
  fix = "eslint --fix {{files}}"
}
```

- `glob` filters the files selected for the run. With no match, the step is skipped.
- `check` should return a nonzero status for problems and leave files unchanged.
- `fix` should apply available fixes and report any problems that remain.
- `{{files}}` expands to the selected file arguments.

A step without file patterns can run even when no files are selected. Use that for whole-project commands, and declare ordering when they read or write beyond a known file set.

### Step commands

Step commands such as `check`, `check_list_files`, `check_diff`, and `fix` accept either a shell command string or a structured `Command`.

String commands run through a shell. Use them when the command needs shell features such as pipes, redirects, `&&`, variable expansion, or glob expansion:

```pkl
check = "eslint {{files}} | tee eslint.log"
```

Use a structured command to execute a program directly, without a shell:

```pkl
check = new Command {
    argv = List("wc", "-c", "{{files}}")
}
```

The first `argv` entry is the executable, which hk resolves using `PATH`. Each remaining entry is passed to the program as one argument after template rendering. Exact, standalone `{{files}}` and `{{workspace_files}}` entries are special: hk expands them into one argument per file. `{{workspace_files}}` contains paths relative to the matched workspace when `workspace_indicator` is configured.

Structured commands preserve argument boundaries, so filenames containing spaces or shell metacharacters are passed literally. Shell syntax is not interpreted: entries such as `"*"`, `"$HOME"`, `"|"`, and `">"` remain literal arguments. Use a string command if shell interpretation is required.

Structured commands cannot be combined with the step's `shell` option or a string
`prefix`. Use an argv-list prefix such as `List("mise", "x", "--")` when the
structured command should run through a launcher. Other step behavior, including
`dir`, `env`, and automatic batching for large file lists, continues to apply.

### Step working directory

`dir` sets the directory a step's commands run in. It is rendered as a template, so a step with `workspace_indicator` can follow each job's workspace rather than opening every command with a `cd`:

```pkl
local linters = new Mapping<String, Step> {
    ["go-vet"] {
        glob = "**/*.go"
        workspace_indicator = "go.mod"
        dir = "{{workspace}}"
        check = "go vet ./..."
    }
}
```

hk creates one job per matched workspace, so this runs `go vet ./...` in `packages/api`, then in `packages/worker`, and so on. Because the `cd` is gone, the command no longer needs a shell and can be written as a structured `Command`.

`{{files}}` is relative to the rendered directory, the same as it already is for a literal `dir`.

File selection happens before hk knows which workspace a job will run in, so `glob` matching, `exclude`, and `stage` pathspecs use only the literal part of `dir` that precedes the first template expression — `sub/{{workspace}}` scopes them to `sub`, and `{{workspace}}` scopes them to nothing. Use `glob` and `workspace_indicator` to select files for a step with a fully templated `dir`.

For commands run with a literal `dir`, `{{workspace}}` and
`{{workspace_indicator}}` are relative to that directory, just like `{{files}}`.
For example, a command running in `packages/api` sees `.` and `go.mod` rather
than `packages/api` and `packages/api/go.mod`.

`stage` patterns are handled separately. Staging runs once per step, after every job, so hk re-resolves a templated `dir` against each matched workspace: `stage = List("generated/**")` stages `packages/a/generated/...` and `packages/b/generated/...`, and leaves a same-named path at the repo root alone. If no workspace matches, the patterns fall back to the repo root and hk warns.

One caveat: while rendering `dir` itself, `{{workspace}}` is relative to the repo
root, never to a subproject. A subproject config that sets a templated `dir`
therefore resolves to the wrong path. hk reports it as a missing working
directory rather than failing obscurely; use a literal `dir` in subprojects for
now.

### Focus checks on failing files

For tools whose detailed `check` output cannot identify failing files in a machine-readable form, set `check_failed_files = true` and provide either `check_list_files` or `check_diff`:

```pkl
local linters = new Mapping<String, Step> {
    ["my-linter"] {
        glob = List("**/*.py")
        check_list_files = "my-linter --list-failing-files {{files}}"
        check = "my-linter check {{files}}"
        fix = "my-linter fix {{files}}"
        check_failed_files = true
    }
}
```

In check mode, hk first runs `check_diff` or `check_list_files` over the complete job. If that command reports a failure, hk extracts and deduplicates the affected paths, then runs `check` only on those files so its full diagnostics remain available without rendering every input path again. If both file-reporting commands are configured, `check_diff` takes precedence.

This behavior is opt-in because it adds another process invocation and requires `check` to accept file arguments. Enabling it requires `check` and at least one of `check_diff` or `check_list_files`. Paths not present in the original job are ignored, focused commands retain automatic argument-limit batching, and a failure from the file-reporting command remains authoritative if the focused check unexpectedly succeeds.

For partial fixers, set `check_after_diff = true` alongside `check` and `check_diff`. After applying a nonempty diff in fix mode, hk reruns `check` on the original batch so non-fixable findings are not hidden by a successfully applied patch. Complete formatters can leave this disabled to retain the single-command fast path.

### Customize a builtin

```pkl
["prettier"] = (Builtins.prettier) {
  glob = List("*.js", "*.ts", "*.json")
  exclude = List("**/generated/**")
}
```

The amended object keeps properties you do not override. See [builtins](/builtins) for the catalogue and command details.

### Dependencies and groups

Use `depends` when the result of one step is needed by another:

```pkl
["prettier"] = (Builtins.prettier) {
  depends = "eslint"
}
```

This waits for the `eslint` step. File locking already prevents simultaneous writes to selected files; a dependency additionally establishes their order.

Prefer the step’s `stage` setting over running `git add` inside a command; hk serializes its own index writes. Serialize commands that write the index themselves with `exclusive`, `depends`, or a group.

A `Group` is a scheduling boundary. Its child steps can run together, but the group waits for prior work and blocks later work until it finishes. Prefer individual dependencies when only a few steps need an order.

#### Group defaults {#group}

```pkl
local frontend = new Group {
  dir = "frontend"
  prefix = List("mise", "x", "--")
  steps {
    ["prettier"] = Builtins.prettier
    ["eslint"] = Builtins.eslint
  }
}
```

Groups can provide `dir`, `prefix`, `workspace_indicator`, `shell`, `stage`, and `exclude`. A child inherits a value only when it does not define its own. Child values replace group values; lists are not merged. A builtin may already define a property, so inspect its definition before relying on inheritance.

### Profiles

Profiles select optional steps:

```pkl
["typecheck"] = (Builtins.tsc) {
  profiles = List("slow")
}
```

```sh
hk check --slow
hk check --profile slow
HK_PROFILE=slow hk check
```

A step requires **all** of its positive profile names to be enabled. `profiles = List("ci", "slow")` requires both `ci` and `slow`. A negative profile such as `"!slow"` prevents that step from running when `slow` is enabled. Quote `!slow` when passing it through a shell.

Set active profiles at the top level, via CLI flags, Git config, or `HK_PROFILE`. A hook’s `env` block configures child commands; it is not the place to select hk’s profiles.

### Workspaces

Use `workspace_indicator` for a tool that works on a project identified by a file:

```pkl
["cargo-clippy"] = (Builtins.cargo_clippy) {
  workspace_indicator = "Cargo.toml"
  check = "cargo clippy --manifest-path {{workspace_indicator}}"
}
```

hk partitions selected files by the matching workspace. `{{workspace}}` is its directory, `{{workspace_indicator}}` is the marker’s path, and `{{workspace_files}}` contains paths relative to that directory.

See the [monorepo example](/reference/examples/monorepo) for component groups and working directories.

### Subprojects

In a monorepo, the root config can load an `hk.pkl` owned by each component:

```pkl
subprojects = List("frontend", "backend", "packages/*")
```

Subproject paths are relative to the root config and may be literal directories or
glob patterns. hk merges a subproject's steps into the root hook with the same name,
then scopes their working directories and file matching to that subproject. A step
named `eslint` in `frontend/hk.pkl` is exposed as `frontend:eslint` for `--step` and
`skip_steps`.

Keep these composition rules in mind:

- Hooks are not copied between events. A subproject step under `check` does not also
  run in `pre-commit` or `fix`; add it to every event where it should run.
- Hook-wide behavior such as `fix`, `stash`, `stage`, and `report` should be set in
  the root config. Subprojects contribute steps and their local environment.
- Subprojects are loaded one level deep. A `subprojects` declaration inside a
  subproject config is ignored with a warning.
- A subproject's literal `dir` is relative to that subproject. Templated workspace
  directories have an additional caveat described under
  [Step working directory](#step-working-directory).

See the complete [monorepo example](/reference/examples/monorepo#nested-configs-with-subprojects),
including per-directory mise environments and locally installed Node tools.

### Conditions and Git status

`condition` is an expression evaluated per step job. `step_condition` is evaluated once per step. Shell commands need an explicit `exec(...)` call:

```pkl
condition = "exec('test -f .lint-enabled')"
```

The `git` object makes common status checks available without invoking Git:

```pkl
condition = "git.staged_files != []"
```

To require a staged Cargo manifest:

```pkl
condition = #"any(git.staged_files, {hasSuffix(#, "Cargo.toml")})"#
```

Available lists include `staged_files`, `unstaged_files`, `untracked_files`, and `modified_files`. Staged classifications include `staged_added_files`, `staged_modified_files`, `staged_deleted_files`, `staged_renamed_files`, and `staged_copied_files`. Unstaged classifications include `unstaged_modified_files`, `unstaged_deleted_files`, and `unstaged_renamed_files`.

These paths are repository-relative. Git status lists are also available to command templates, for example `{{ git.staged_files }}`.

## Configuration precedence

Runtime settings resolve from lowest to highest precedence:

| Precedence | Source                                                               |
| ---------- | -------------------------------------------------------------------- |
| 1          | Built-in defaults                                                    |
| 2          | User configuration, typically `~/.config/hk/config.pkl`              |
| 3          | Selected project configuration                                       |
| 4          | Git configuration, with local values overriding global/system values |
| 5          | `HK_*` environment variables                                         |
| 6          | CLI flags                                                            |

Higher layers override lower ones for scalar settings. List settings such as `exclude`, `skip_steps`, `skip_hooks`, and `hide_warnings` combine values across sources.

### User configuration {#hkrc}

Use `~/.config/hk/config.pkl` for defaults and additional steps across projects. The location follows `XDG_CONFIG_HOME` or `HK_CONFIG_DIR` when set.

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Config.pkl"

jobs = 4
fail_fast = false
skip_steps = List("optional-check")
```

For user files amending `Config.pkl`, hooks and steps merge additively with the project: user configuration adds names the project does not define, and project definitions win on collisions. Use `hk.local.pkl` to replace project behavior locally.

::: warning Legacy user schema
Files amending `UserConfig.pkl` use older precedence: their `display_skip_reasons`, `hide_warnings`, `warnings`, `stage`, and environment values override the project. Amend `Config.pkl` to use the precedence described above.
:::

::: warning Legacy hkrc paths
`.hkrc.pkl`, `~/.hkrc.pkl`, and `--hkrc` are deprecated. Discovery still checks the legacy files before the user configuration directory. Migrate project overrides to `hk.local.pkl` and shared user defaults to `~/.config/hk/config.pkl`.
:::

Global configuration is separate from [global hook installation](/getting_started#install-hooks). An installed hook in a repository without a project configuration exits silently.

### Git configuration

Use Git settings for persistent preferences without modifying `hk.pkl`:

```sh
git config --local hk.jobs 4
git config --local hk.skipSteps "slow-test,noisy-formatter"
git config --local hk.skipHook pre-push
git config --global hk.failFast false
```

List settings accept comma-separated values or multiple Git entries:

```sh
git config --local hk.exclude node_modules
git config --local --add hk.exclude "**/*.min.js"
```

### Inspect effective settings

```sh
hk config dump
hk config get exclude
hk config explain jobs
```

These commands inspect runtime settings. To inspect hook execution, use `hk check --plan`; to evaluate the Pkl file, use `hk validate` or the optional Pkl CLI.

## Schema reference

The following reference is generated from the schema’s documentation. It covers top-level configuration, hooks, steps, and groups.

<!--@include: ./gen/pkl-config.md-->

## Settings reference

Each setting below lists its type, default, and supported sources. Pkl property names use underscores; CLI flags generally use hyphens.

<!--@include: ./gen/settings-config.md-->
