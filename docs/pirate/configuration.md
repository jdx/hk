---
outline: deep
description: Chart yer ship. Configure hooks, steps (the hands), file selection (the cargo), profiles (the watches), local overrides, and runtime settings.
sourceHash: 4b073aa60724
---

# Configuration, the ship's charts

hk reads `hk.pkl` to decide which hands (steps) to call and how to work them. Start with a shared set of lookouts (linters), then add file filters, dependencies, and profiles as yer ship needs them. As the song has it, hook, helm and harbour-master "steer by the one set of charts, in hk.pkl, typed and true."

Getting under way for the first time? Follow [getting started](/getting_started). For whole configurations, fully rigged, see the [examples](/reference/examples/).

## `hk.pkl`, the charts ye sail by {#hk-pkl}

A configuration amends hk's [Pkl schema](/pkl_introduction). For one set of lookouts that every hook shares, prefer top-level `steps`:

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Builtins.pkl"

steps {
  ["eslint"] = Builtins.eslint
  ["prettier"] = Builtins.prettier
}
```

`pre-commit` mends the staged files while yer unstaged work is stowed safe in the hold. The `check` and `fix` hooks give ye the local commands. Mind: they be hooks, not individual steps.

### Hooks that come rigged by default {#hook-defaults}

Top-level `steps` is optional. When it isn't empty, it rigs the `check`, `fix`, and `pre-commit` hooks. `check`
inspects the cargo: it runs checks, with no fixing and no staging. `fix` mends the canvas, with no staging.
`pre-commit` mends, stages the resulting changes, and stows with Git
stashing by default, so yer unstaged work comes back up from the hold after the hook runs.

If ye configure a hook by one of those names yerself, it keeps its own
hook-level settings, and its steps replace inherited steps of the same name;
top-level steps still supply the rest of the step names. Any other hook name is
a custom hook, and ye must declare it explicitly under `hooks` with its own
steps. Set `fix` or `stage` only when ye need them; custom hooks are unstaged
by default.

Ye can leave out top-level `steps` and muster steps only inside hooks. That's fully supported in v2, and handy when each hook needs a different crew:

```pkl
hooks {
  ["check"] {
    steps {
      ["eslint"] = Builtins.eslint
    }
  }
  ["pre-commit"] {
    fix = true
    stage = true
    stash = "git"
    steps {
      ["prettier"] = Builtins.prettier
    }
  }
}
```

Without top-level steps, hk does not rig the three default hooks. The example above declares only `check` and `pre-commit`; declare a `fix` hook too if ye want `hk fix`. Typed mappings already in yer charts, such as `local linters = new Mapping<String, Step> { ... }` and `steps = linters`, are still supported.

### Where hk looks for the charts {#config-file-paths}

hk starts in the current directory and climbs upward, like a hand going up the rigging. At each directory it checks these paths in order and takes the first one it finds:

| Order | Path                   | What it's for                           |
| ----- | ---------------------- | --------------------------------------- |
| 1     | `hk.local.pkl`         | Yer own local override for the ship     |
| 2     | `.config/hk.local.pkl` | Yer own local override under `.config/` |
| 3     | `hk.pkl`               | The charts all yer shipmates share      |
| 4     | `.config/hk.pkl`       | The shared charts under `.config/`      |

[`HK_FILE`](/environment_variables#hk-file) points hk at a specific configuration instead. hk sails by one project file; it does not merge every file it finds.

### `hk.local.pkl`, yer own marks on the charts {#hk-local-pkl}

Use Pkl's `amends` to extend the shared project configuration locally:

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

Add `hk.local.pkl` to `.git/info/exclude` or the project's `.gitignore`. This example keeps the inherited steps and signs on one more hand. Assign a new mapping when ye want to replace the hook's explicitly declared step list:

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

Even after a hook's step mapping is replaced, top-level steps still supply any names it's missing. To replace every shared hand in a local configuration, ye must replace the top-level `steps` mapping too.

## Sign on a hand: define a step {#define-a-step}

A step picks its cargo (files) and declares its commands:

```pkl
local eslint = new Step {
  glob = List("*.js", "*.ts")
  exclude = List("**/generated/**")
  check = "eslint {{files}}"
  fix = "eslint --fix {{files}}"
}
```

- `glob` filters the cargo selected for the run. If nothing matches, the step is skipped.
- `check` should return a nonzero status when it spots trouble, and leave the files unchanged. A lookout only looks.
- `fix` should mend what it can and report any problems that remain.
- `{{files}}` expands to the selected files, as arguments.

A step without file patterns can run even when no cargo is selected. Use that for commands that work the whole ship, and declare their ordering when they read or write beyond a known set of files.

### A hand's duties: step commands {#step-commands}

A hand's commands, such as `check`, `check_list_files`, `check_diff`, and `fix`, take either a shell command string or a structured `Command`.

String commands run through a shell. Use one when yer command needs shell features such as pipes, redirects, `&&`, variable expansion, or glob expansion:

```pkl
check = "eslint {{files}} | tee eslint.log"
```

Use a structured command to run a program directly, with no shell in between:

```pkl
check = new Command {
    argv = List("wc", "-c", "{{files}}")
}
```

The first `argv` entry is the executable, and hk seeks it out on the `PATH`. Every other entry is handed to the program as one argument, after template rendering. Entries that are exactly `{{files}}` or `{{workspace_files}}`, standing alone, are special: hk expands them into one argument per file. When `workspace_indicator` is configured, `{{workspace_files}}` holds paths relative to the matched workspace.

Structured commands keep the boundaries between arguments, so filenames with spaces or shell metacharacters are passed exactly as they are. No shell meddles with them: entries such as `"*"`, `"$HOME"`, `"|"`, and `">"` stay literal arguments. If ye need the shell to interpret something, use a string command.

Structured commands can't sail together with the step's `shell` option or a
string `prefix`. When a structured command should run through a launcher, use an
argv-list prefix such as `List("mise", "x", "--")`. The rest of a step's
behaviour still applies, including `dir`, `env`, and automatic batching for
large file lists.

### Literal braces that must sail through {#literal-braces-in-commands}

Commands are rendered as [Tera](https://keats.github.io/tera/) templates, so `{{` starts an expression. A tool whose own syntax uses `{{`, such as a Go template, runs aground here, because the command fails to render:

```pkl
// error: "{{.ResourceKind}}" is parsed as a hk expression
check = "kubeconform -schema-location 'https://example.com/{{.ResourceKind}}.json' {{files}}"
```

Wrap the literal part in `{% raw %}` and it passes through unchanged:

```pkl
check = "kubeconform -schema-location 'https://example.com/{% raw %}{{.ResourceKind}}{% endraw %}.json' {{files}}"
```

### Where a hand stands: step working directory {#step-working-directory}

`dir` sets the directory a step's commands run in. It's rendered as a template, so a step with `workspace_indicator` can follow each job to its own workspace, instead of opening every command with a `cd`:

```pkl
local linters = new Mapping {
    ["go-vet"] {
        glob = "**/*.go"
        workspace_indicator = "go.mod"
        dir = "{{workspace}}"
        check = "go vet ./..."
    }
}
```

hk creates one job per matched workspace, so this runs `go vet ./...` in `packages/api`, then in `packages/worker`, and so on. With the `cd` gone overboard, the command no longer needs a shell and can be written as a structured `Command`.

`{{files}}` takes its bearings from the rendered directory: its paths are relative to it, just as they already are for a literal `dir`.

hk picks the cargo before it knows which workspace a job will run in. So `glob` matching, `exclude`, and `stage` pathspecs use only the literal part of `dir` that comes before the first template expression: `sub/{{workspace}}` scopes them to `sub`, and `{{workspace}}` scopes them to nothing at all. For a step with a fully templated `dir`, use `glob` and `workspace_indicator` to select its files.

For commands run with a literal `dir`, `{{workspace}}` and
`{{workspace_indicator}}` are relative to that directory, same as `{{files}}`.
A hand's command running in `packages/api`, for example, sees `.` and `go.mod`, not
`packages/api` and `packages/api/go.mod`.

`stage` patterns are handled on their own. Staging runs once per step, after every job is done, so hk resolves a templated `dir` again against each matched workspace: `stage = List("generated/**")` stages `packages/a/generated/...` and `packages/b/generated/...`, and leaves a path of the same name at the repo root alone. If no workspace matches, the patterns fall back to the repo root and hk sings out a warning.

One caveat, and heed it: while rendering `dir` itself, `{{workspace}}` is
relative to the repo root, never to a subproject. So a subproject config that
sets a templated `dir` resolves to the wrong path. hk reports it as a missing
working directory rather than failing in some murky way; for now, use a literal
`dir` in subprojects.

### Focus the lookouts on failing files {#focus-checks-on-failing-files}

Some lookouts' detailed `check` output can't name the failing files in a form a machine can read. For those tools, set `check_failed_files = true` and provide either `check_list_files` or `check_diff`:

```pkl
local linters = new Mapping {
    ["my-linter"] {
        glob = List("**/*.py")
        check_list_files = "my-linter --list-failing-files {{files}}"
        check = "my-linter check {{files}}"
        fix = "my-linter fix {{files}}"
        check_failed_files = true
    }
}
```

In check mode, hk first runs `check_diff` or `check_list_files` over the whole job. If that command reports a failure, hk pulls out the affected paths, drops the duplicates, then runs `check` on only those files. Ye still get its full diagnostics, without every input path rendered again. If both file-reporting commands are configured, `check_diff` takes precedence.

This is off unless ye ask for it, because it adds another process invocation and requires `check` to accept file arguments. Turning it on requires `check` and at least one of `check_diff` or `check_list_files`. Paths that weren't in the original job are ignored, and focused commands keep automatic argument-limit batching. If the focused check unexpectedly passes, the failure from the file-reporting command still stands, and the step still fails.

For partial fixers, sailmakers who can't mend every tear, set `check_after_diff = true` alongside `check` and `check_diff`. After applying a nonempty diff in fix mode, hk runs `check` again on the original batch, so a patch that applied cleanly can't hide the findings it couldn't fix. Complete formatters can leave this off and keep the single-command fast path.

A tool that writes its fixes up in a SARIF log can get a `check_diff` from `hk util sarif-diff`, which runs the tool and turns its fixes into a patch, as the `pinact` builtin does. If any result has no fix, it prints no patch at all, so hk runs `fix` and the finding that can't be fixed is still reported, never lost overboard:

```pkl
check_diff = "hk util sarif-diff -- pinact run --check --format sarif {{files}}"
```

A quick sailmaker, one that only rewrites the files that need it and runs faster than hk can capture and apply a patch, can set `apply_check_diff = false`. Fix mode then runs `fix` instead of applying the diff, and `check_diff` still shows the diff in check mode. In a hook that stages fixes, hk still runs `check_diff` first and hands `fix` only the files the diff names, so a passing check skips both the fixer and the staging.

A formatter that can read a file on stdin and print the formatted result needs no diff mode of its own. `hk util format-diff` runs it once per file, in parallel, all hands at once, and prints the patch, as the `stylua`, `tombi_format`, `buildifier_format`, `terraform`, and `tofu` builtins do. `{}` stands for the file's path:

```pkl
check_diff = "hk util format-diff {{files}} -- stylua --stdin-filepath {} -"
```

If the formatter fails on any file, no patch is printed and hk runs `fix`, which reports the error. Beware: a formatter's stdin mode can ignore excludes in its own configuration that it does apply to files named on the command line, as yamlfmt's and taplo's do. Their builtins ask the tool which files would change before formatting those.

### What the lookouts sang out: diagnostics {#diagnostics}

`hk check --sarif`, the `diagnostics` arrays in `--format json` and `--format jsonl` output, and the MCP dashboard all show normalized diagnostics: findings with a file, position, severity, message, and rule. hk builds them by parsing the output of a hand's `check` command, and it can't guess a tool's output format. A hand reports diagnostics only when it sets `diagnostic_format`. Without it the hand still runs, fails, and keeps its raw `output` in the results (unless `output_summary = "hide"`), but its `diagnostics` list and its SARIF results are empty. In `--format jsonl` output, the findings are in the final `run_completed` result; each `step_completed` event carries an empty `diagnostics` array.

The standing crew are no different. A builtin sets `diagnostic_format` only when the tool's default output is one of the formats below, because hk doesn't add flags that would change what the tool prints. Most builtins don't set it (check a builtin's definition in `pkl/builtins`), so an unchanged builtin hand sings out no diagnostics even when it fails. To get diagnostics from one of those, set `diagnostic_format` on yer own hand, and add the tool's flag for a supported format to its `check` command if ye accept the output changing.

| `diagnostic_format` | What hk reads                                                                                                                                                             |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `gcc`               | Lines like `path:line:column: warning: message [rule]`. The severity and the trailing `[rule]` are optional, and lines that follow a diagnostic are added to its message. |
| `sarif`             | A SARIF 2.1.0 log. Each result becomes a diagnostic, with its rule and help link.                                                                                         |
| `eslint-json`       | The JSON array that `eslint --format json` prints.                                                                                                                        |
| `cargo-json`        | The stream that `cargo check --message-format=json` prints. Each `compiler-message` becomes a diagnostic.                                                                 |

hk parses the combined stdout and stderr of `check` runs. If a hand captured no `check` output, structured results fall back to the hand's retained `output`, so a failing `fix` command's output can also be parsed. `diagnostic_tool` sets the tool name recorded on each diagnostic, which defaults to the step name. The raw output stays in the hand's `output` as `output_summary` allows; with `output_summary = "hide"` and no `diagnostic_format`, none is kept. Output that can't be parsed usually becomes an entry in the hand's `parse_warnings`, but not always: the `gcc` parser appends an unrecognized line that follows a diagnostic to that diagnostic's message, and the `cargo-json` parser skips valid JSON events that aren't `compiler-message`. This hand reports each line of the compiler's `path:line:column: message` output as a diagnostic:

```pkl
["compiler"] {
    check = "my-compiler {{files}}"
    diagnostic_format = "gcc"
    diagnostic_tool = "my-compiler"
}
```

### Refit a standing hand: customize a builtin {#customize-a-builtin}

```pkl
["prettier"] = (Builtins.prettier) {
  glob = List("*.js", "*.ts", "*.json")
  exclude = List("**/generated/**")
}
```

The amended object keeps every property ye don't override. See [builtins](/builtins) for the full crew roster and command details.

### Who hauls first: dependencies and groups {#dependencies-and-groups}

Use `depends` when one hand needs another hand's work done first:

```pkl
["prettier"] = (Builtins.prettier) {
  depends = "eslint"
}
```

This waits for the `eslint` step. The lashings (file locks) already keep two hands from writing the same selected file at once; a dependency also sets the order they work in.

Prefer the step's `stage` setting over running `git add` inside a command; hk takes its own index writes one at a time. Commands that write the index themselves, ye must serialize with `exclusive`, `depends`, or a group.

A `Group` is a scheduling boundary, a gang of hands that goes aloft together. Its child steps can haul at once, but the group waits for the work before it, and holds back later work until it finishes. When only a few steps need an order, prefer individual dependencies.

#### What a gang hands down: group defaults {#group}

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

Groups can provide `dir`, `prefix`, `workspace_indicator`, `shell`, `stage`, and `exclude`. A child inherits a value only when it doesn't define its own. Child values replace group values; lists are not merged. A builtin may already define a property, so inspect its definition before ye rely on inheritance.

### The watches: profiles {#profiles}

Profiles, the ship's watches, select optional steps:

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

A step turns out only when **all** of its positive profile names are enabled. `profiles = List("ci", "slow")` requires both the `ci` and the `slow` watch. A negative profile such as `"!slow"` keeps that step from running when `slow` is enabled. Quote `!slow` when passing it through a shell.

Set the watches on duty at the top level, with CLI flags, Git config, or `HK_PROFILE`. A hook's `env` block configures child commands; it is not the place to select hk's profiles, matey.

### Workspaces: many cabins, one ship {#workspaces}

Use `workspace_indicator` for a tool that works on a project marked out by a file, like a cabin known by the name on its door:

```pkl
["cargo-clippy"] = (Builtins.cargo_clippy) {
  workspace_indicator = "Cargo.toml"
  check = "cargo clippy --manifest-path {{workspace_indicator}}"
}
```

hk sorts the selected cargo by its matching workspace. `{{workspace}}` is that workspace's directory, `{{workspace_indicator}}` is the marker's path, and `{{workspace_files}}` holds paths relative to that directory.

See the [monorepo example](/reference/examples/monorepo) for component groups and working directories.

### Subprojects: charts for every quarter of the ship {#subprojects}

In a monorepo, the root config can take aboard an `hk.pkl` that each component keeps:

```pkl
subprojects = List("frontend", "backend", "packages/*")
```

Subproject paths are relative to the root config, and can be literal directories
or glob patterns. hk merges a subproject's steps into the root hook of the same
name, then scopes their working directories and file matching to that
subproject. A step named `eslint` in `frontend/hk.pkl` answers to
`frontend:eslint` for `--step` and `skip_steps`.

Mind these rules for joining the charts together:

- Hooks are not copied between events. A subproject step under `check` does not also
  run in `pre-commit` or `fix`; add it to every event where it should run.
- Set hook-wide behaviour such as `fix`, `stash`, `stage`, and `report` in the
  root config, the master chart. Subprojects bring steps and their local environment.
- Subprojects are loaded one level deep. A `subprojects` declaration inside a
  subproject config is ignored, with a warning.
- A subproject's literal `dir` is relative to that subproject. Templated workspace
  directories carry one more caveat, described under
  [Step working directory](#step-working-directory).

See the complete [monorepo example](/reference/examples/monorepo#nested-configs-with-subprojects),
including per-directory mise environments and locally installed Node tools.

### Reading the weather: conditions and Git status {#conditions-and-git-status}

A hand reads the weather before it hauls. `condition` is an expression evaluated for each job of a step. `step_condition` is evaluated once per step. To haul a step only when a shell command succeeds, wrap it in `exec_ok(...)`:

```pkl
condition = "exec_ok('test -f .lint-enabled')"
```

`exec_ok(command)` is true when the command exits with status 0 and false otherwise. `exec(command)` returns the command's standard output as a string, for comparisons such as `exec('git branch --show-current') == 'main\n'`. Use `exec` for its output, not to test success: a command that exits non-zero, or prints output that is not valid UTF-8, makes `exec` fail the hook, and a string result never skips a step.

The `git` object gives ye common status checks without calling on Git itself:

```pkl
condition = "git.staged_files != []"
```

To require a staged Cargo manifest:

```pkl
condition = #"any(git.staged_files, {hasSuffix(#, "Cargo.toml")})"#
```

The lists on hand include `staged_files`, `unstaged_files`, `untracked_files`, and `modified_files`. Staged classifications include `staged_added_files`, `staged_modified_files`, `staged_deleted_files`, `staged_renamed_files`, and `staged_copied_files`. Unstaged classifications include `unstaged_modified_files`, `unstaged_deleted_files`, and `unstaged_renamed_files`.

These paths are relative to the ship (the repository). Git status lists are also available to command templates, for example `{{ git.staged_files }}`.

The bosun's pipe brings word of its own too: Git hook arguments such as `hook_args`, `commit_msg_file`, and `is_branch_checkout` are also available as condition variables. See [other Git events](/hooks#other-git-events) for the variables each hook provides.

Mind: conditions are expr-lang expressions, not Tera templates. Name variables directly, as in `is_branch_checkout`, not as `{{ is_branch_checkout }}`.

## Who outranks whom: configuration precedence {#configuration-precedence}

Runtime settings are settled from lowest precedence to highest, like a chain of command:

| Precedence | Where it hails from                                                  |
| ---------- | -------------------------------------------------------------------- |
| 1          | Built-in defaults                                                    |
| 2          | User configuration, typically `~/.config/hk/config.pkl`              |
| 3          | The selected project configuration                                   |
| 4          | Git configuration, with local values overriding global/system values |
| 5          | `HK_*` environment variables, the standing orders                    |
| 6          | CLI flags, flown on the call itself                                  |

For scalar settings, a higher layer's value overrides the ones below it. List settings such as `exclude`, `skip_steps`, `skip_hooks`, and `hide_warnings` are different: they gather up their values from every source.

### Yer own sea chest: user configuration {#hkrc}

Use `~/.config/hk/config.pkl` for defaults and extra steps that sail with ye on every project. The location follows `XDG_CONFIG_HOME` or `HK_CONFIG_DIR` when set.

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"

jobs = 4
fail_fast = false
skip_steps = List("optional-check")
```

For user files amending `Config.pkl`, hooks and steps merge additively with the project: yer user configuration adds names the project doesn't define, and when names collide, the project's definitions win. Use `hk.local.pkl` to replace project behaviour locally.

For removed `UserConfig.pkl` fields (struck off) and legacy paths, see the
[hk v2 migration guide](/migration-v2).

Global configuration is a separate matter from [global hook installation](/getting_started#install-hooks). An installed hook in a repository with no project configuration exits silently: no charts, no call.

### Settings kept in Git: Git configuration {#git-configuration}

Use Git settings for preferences that stick, without touching `hk.pkl`:

```sh
git config --local hk.jobs 4
git config --local hk.skipSteps "slow-test,noisy-formatter"
git config --local hk.skipHook pre-push
git config --global hk.failFast false
```

List settings take comma-separated values or several Git entries, as ye like:

```sh
git config --local hk.exclude node_modules
git config --local --add hk.exclude "**/*.min.js"
```

### What the ship sails by right now: inspect effective settings {#inspect-effective-settings}

```sh
hk config dump
hk config get exclude
hk config explain jobs
```

These commands inspect runtime settings. To inspect how hooks will run (the passage plan), use `hk check --plan`; to evaluate the Pkl file, use `hk validate` or the optional Pkl CLI.

## The chart-maker's key: schema reference {#schema-reference}

The reference below is generated from the schema's own documentation, so it stays in plain English. It covers top-level configuration, hooks, steps, and groups.

<!--@include: ../gen/pkl-config.md-->

## Every setting the ship sails by: settings reference {#settings-reference}

Each setting below lists its type, its default, and the sources it can come from; it's generated too, and stays in plain English. Pkl property names use underscores; CLI flags generally use hyphens.

<!--@include: ../gen/settings-config.md-->
