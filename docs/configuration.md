---
outline: deep
description: Configure hooks, steps, file selection, profiles, local overrides, and runtime settings.
---

# Configuration

hk reads `hk.pkl` to decide which steps to run and how to run them. Start with a shared set of linters, then add file filters, dependencies, and profiles as your project needs them.

For a first setup, use [getting started](/getting_started). For complete configurations, see the [examples](/reference/examples/).

## `hk.pkl`

A configuration amends hk’s [Pkl schema](/pkl_introduction). For a shared set of linters, prefer top-level `steps`:

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Builtins.pkl"

steps {
  ["eslint"] = Builtins.eslint
  ["prettier"] = Builtins.prettier
}
```

`pre-commit` applies fixes to staged files while unstaged work is saved. The `check` and `fix` hooks provide the local commands. They are hooks, not individual steps.

### Hook defaults {#hook-defaults}

Top-level `steps` is optional. When nonempty, it creates the `check`, `fix`, and `pre-commit` hooks. `check`
runs checks without fixing or staging. `fix` applies fixes without staging.
`pre-commit` applies fixes, stages the resulting changes, and defaults to Git
stashing so unstaged work is restored after the hook runs.

An explicitly configured hook with one of those names keeps its hook-level
settings and replaces same-named inherited steps; top-level steps still supply
the remaining step names. Other hook names are custom hooks and must be declared
explicitly under `hooks` with their own steps. Configure `fix` or `stage` only
when needed; custom hooks are unstaged by default.

You can omit top-level `steps` and define steps only inside hooks. This is fully supported in v2 and is useful when each hook needs a different set of steps:

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

Without top-level steps, hk does not create the three default hooks. The example above declares only `check` and `pre-commit`; declare a `fix` hook too if you want `hk fix`. Existing typed mappings such as `local linters = new Mapping<String, Step> { ... }` and `steps = linters` remain supported.

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

hk selects `hk.local.pkl` instead of `hk.pkl` rather than merging the two, so a local file without `amends "./hk.pkl"` drops the shared configuration entirely. hk warns when that happens; hide the warning with `HK_HIDE_WARNINGS=local-config-replaces-shared`.

Add `hk.local.pkl` to `.git/info/exclude` or the project’s `.gitignore`. This example preserves inherited steps and adds one. Assign a new mapping when you want to replace the hook’s explicitly declared step list:

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

Top-level steps still supply missing names after a hook’s step mapping is replaced. To replace all shared steps for a local configuration, replace the top-level `steps` mapping too.

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

### Shell commands on Windows

On Windows, a string command runs through `cmd.exe` unless the step sets `shell`. A command that is a plain program with arguments behaves the same there, but POSIX shell syntax such as `$(...)`, `for` loops, `[ ... ]`, `trap`, or `/dev/null` does not. A few builtins are written as POSIX scripts and need a POSIX shell on Windows: `go_fmt`, `go_imports`, `jq`, `pkl`, `terraform_docs`, `terraform_validate`, `terragrunt_hcl_fmt`, `terragrunt_hcl_validate`, `tf_lint`, `typos`, and `yq`. `just_format` also needs a POSIX shell, with `xargs`, and `nix_fmt` has no Windows commands. Other builtins run as they are.

To run one of them, set its `shell` to a POSIX shell, such as the `sh` that comes with Git for Windows. A value that contains quotes groups the quoted words, so a path with spaces can be quoted (on Windows backslashes stay literal, except before a double quote, as in `CommandLineToArgvW`):

```pkl
["jq"] = (Builtins.jq) {
    shell = "\"C:/Program Files/Git/usr/bin/sh.exe\" -o errexit -c"
}
```

If `sh` is already on `PATH`, `shell = "sh -o errexit -c"` is enough. Include `-o errexit -c`: it is what hk uses on other platforms, and the scripts rely on it.

`go_imports` also defines a structured `fix`, which cannot be combined with `shell`. Replace it with a string command as well, keeping its write effect:

```pkl
["go_imports"] = (Builtins.go_imports) {
    shell = "sh -o errexit -c"
    fix = new CommandSpec {
        command = "goimports -w {{files}}"
        effect = "write"
    }
}
```

Set `shell` on the step itself, not on a group that also holds steps with structured commands: a step cannot combine `shell` with a structured `Command`, and a group passes its `shell` to every step that has none.

### Literal braces in commands

Commands are rendered as [Tera](https://keats.github.io/tera/) templates, so `{{` starts an expression. A tool whose own syntax uses `{{`, such as a Go template, fails to render:

```pkl
// error: "{{.ResourceKind}}" is parsed as a hk expression
check = "kubeconform -schema-location 'https://example.com/{{.ResourceKind}}.json' {{files}}"
```

Wrap the literal part in `{% raw %}` to pass it through unchanged:

```pkl
check = "kubeconform -schema-location 'https://example.com/{% raw %}{{.ResourceKind}}{% endraw %}.json' {{files}}"
```

### Step working directory

`dir` sets the directory a step's commands run in. It is rendered as a template, so a step with `workspace_indicator` can follow each job's workspace rather than opening every command with a `cd`:

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

In check mode, hk first runs `check_diff` or `check_list_files` over the complete job. If that command reports a failure, hk extracts and deduplicates the affected paths, then runs `check` only on those files so its full diagnostics remain available without rendering every input path again. If both file-reporting commands are configured, `check_diff` takes precedence.

This behavior is opt-in because it adds another process invocation and requires `check` to accept file arguments. Enabling it requires `check` and at least one of `check_diff` or `check_list_files`. Paths not present in the original job are ignored, focused commands retain automatic argument-limit batching, and a failure from the file-reporting command remains authoritative if the focused check unexpectedly succeeds.

For partial fixers, set `check_after_diff = true` alongside `check` and `check_diff`. After applying a nonempty diff in fix mode, hk reruns `check` on the original batch so non-fixable findings are not hidden by a successfully applied patch. Complete formatters can leave this disabled to retain the single-command fast path.

A tool that reports fixes in a SARIF log can get a `check_diff` from `hk util sarif-diff`, which runs it and turns the fixes into a patch, as the `pinact` builtin does. If any result has no fix, it prints no patch, so hk runs `fix` and the unfixable finding is still reported:

```pkl
check_diff = "hk util sarif-diff -- pinact run --check --format sarif {{files}}"
```

A fixer that only rewrites files that need it, and runs faster than hk can capture and apply a patch, can set `apply_check_diff = false`. Fix mode then runs `fix` instead of applying the diff, and `check_diff` still shows the diff in check mode. In a hook that stages fixes, hk still runs `check_diff` first and passes `fix` only the files the diff names, so a passing check skips both the fixer and staging.

A formatter that can read a file on stdin and print the formatted result doesn't need a diff mode of its own. `hk util format-diff` runs it once per file, in parallel, and prints the patch, as the `stylua`, `tombi_format`, `buildifier_format`, `terraform`, and `tofu` builtins do. `{}` stands for the file's path:

```pkl
check_diff = "hk util format-diff {{files}} -- stylua --stdin-filepath {} -"
```

If the formatter fails for any file, no patch is printed and hk runs `fix`, which reports the error. A formatter's stdin mode can ignore excludes in its configuration that it applies to files named on the command line, as yamlfmt's and taplo's do; their builtins ask the tool which files would change before formatting those.

### Diagnostics {#diagnostics}

`hk check --sarif`, the `diagnostics` arrays in `--format json` and `--format jsonl` output, and the MCP dashboard all show normalized diagnostics: findings with a file, position, severity, message, and rule. hk builds them by parsing the output of a step's `check` command, and it can't guess a tool's output format. A step reports diagnostics only when it sets `diagnostic_format`. Without it the step still runs, fails, and can carry its raw text in the result's `output` field (separate from `diagnostics`; `output_summary` and other step settings decide when it is present), but its `diagnostics` list and its SARIF results are empty. In `--format jsonl` output, the findings are in the final `run_completed` result; each `step_completed` event carries an empty `diagnostics` array.

This applies to builtins too. A builtin sets `diagnostic_format` only when the tool's default output is one of the formats below, because hk doesn't add flags that would change what the tool prints. Most builtins don't set it (check a builtin's definition in `pkl/builtins`), so an unchanged builtin step contribute no diagnostics even when they fail. To report diagnostics for one of those, set `diagnostic_format` on your own step, and add the tool's flag for a supported format to its `check` command if you accept the output changing.

| `diagnostic_format` | What hk reads                                                                                                                                                             |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `gcc`               | Lines like `path:line:column: warning: message [rule]`. The severity and the trailing `[rule]` are optional, and lines that follow a diagnostic are added to its message. |
| `sarif`             | A SARIF 2.1.0 log. Each result becomes a diagnostic, with its rule and help link.                                                                                         |
| `eslint-json`       | The JSON array that `eslint --format json` prints.                                                                                                                        |
| `cargo-json`        | The stream that `cargo check --message-format=json` prints. Each `compiler-message` becomes a diagnostic.                                                                 |

hk parses the combined stdout and stderr of `check` runs. If a step captured no `check` output, structured results fall back to the step's retained `output`, so a failing `fix` command's output can also be parsed. `diagnostic_tool` sets the tool name recorded on each diagnostic, which defaults to the step name. Raw text is reported in the separate `output` field, and `output_summary` and other step settings decide when it is present. Output that can't be parsed usually becomes an entry in the step's `parse_warnings`, but not always: the `gcc` parser appends an unrecognized line that follows a diagnostic to that diagnostic's message, and the `cargo-json` parser skips valid JSON events that aren't `compiler-message`. This step reports each line of the compiler's `path:line:column: message` output as a diagnostic:

```pkl
["compiler"] {
    check = "my-compiler {{files}}"
    diagnostic_format = "gcc"
    diagnostic_tool = "my-compiler"
}
```

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
- A subproject's `skip_steps` name its own steps, and skip only those. A group, or a step
  inside a group, cannot be skipped from a subproject; hk warns when an entry names one. Other
  top-level settings in a subproject config, such as `exclude`, `fail_fast`, `jobs`, or `profiles`,
  have no effect; hk warns that they are ignored. Set them in the root config.
- Subprojects are loaded one level deep. A `subprojects` declaration inside a
  subproject config is ignored with a warning.
- A subproject's literal `dir` is relative to that subproject. Templated workspace
  directories have an additional caveat described under
  [Step working directory](#step-working-directory).

See the complete [monorepo example](/reference/examples/monorepo#nested-configs-with-subprojects),
including per-directory mise environments and locally installed Node tools.

### Conditions and Git status

`condition` is an expression evaluated per step job. `step_condition` is evaluated once per step. To run a step only when a shell command succeeds, wrap it in `exec_ok(...)`:

```pkl
condition = "exec_ok('test -f .lint-enabled')"
```

`exec_ok(command)` is true when the command exits with status 0 and false otherwise. `exec(command)` returns the command’s standard output as a string, for comparisons such as `exec('git branch --show-current') == 'main\n'`. Use `exec` for its output, not to test success: a command that exits non-zero, or prints output that is not valid UTF-8, makes `exec` fail the hook, and a string result never skips a step.

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

Git hook arguments such as `hook_args`, `commit_msg_file`, and `is_branch_checkout` are also available as condition variables. See [Other Git events](/hooks#other-git-events) for the variables each hook provides.

Conditions are expr-lang expressions, not Tera templates. Reference variables directly, as in `is_branch_checkout`, rather than `{{ is_branch_checkout }}`.

## Share configuration across repositories {#share-configuration}

hk has no setting that fetches a configuration from a Git URL. Because `hk.pkl` is a Pkl module, each repository instead amends a shared module and adds what is specific to it. Each repository still evaluates its own configuration and runs its own steps; a change to the shared module reaches a repository when that repository adopts it.

| Pattern                                                  | Version pinned by      | Works offline                            | Needs                                |
| -------------------------------------------------------- | ---------------------- | ---------------------------------------- | ------------------------------------ |
| [Self-hosted package](#share-package)                    | Package version in URL | After the first download, from the cache | Any HTTPS file host                  |
| [Relative `amends` or a Git submodule](#share-submodule) | Commit                 | Yes                                      | Checkout of the shared files         |
| Plain HTTPS module (`amends "https://…/hk.pkl"`)         | Nothing                | No: refetched whenever hk re-evaluates   | Any HTTPS file host; not recommended |

Use a package when many repositories share a configuration and you want versioned releases.

### Publish a versioned package {#share-package}

A package is a ZIP of Pkl files served from an HTTPS URL. This one shares two builtins that need no external tools. Create a project directory with a `PklProject` and the shared module:

```pkl [PklProject]
amends "pkl:Project"

package {
  name = "acme-hk"
  version = "1.0.0"
  baseUri = "package://hk-config.example.com/acme-hk"
  packageZipUrl = "https://hk-config.example.com/acme-hk@\(version).zip"
}
```

```pkl [hk.pkl]
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Builtins.pkl"

steps {
  ["trailing_whitespace"] = Builtins.trailing_whitespace
  ["newlines"] = Builtins.newlines
}
```

Import hk’s schema by its full `package://` URL, as above. hk’s evaluator does not resolve Pkl project dependencies, so the `@name/…` import form that a `dependencies` block provides fails with `Import not found`.

Build the package with the Pkl CLI and upload both files it writes, `acme-hk@1.0.0` and `acme-hk@1.0.0.zip`, to the paths in `baseUri` and `packageZipUrl`:

```sh
pkl project package --output-path out .
```

A repository then adopts the shared configuration and adds its own steps:

```pkl
amends "package://hk-config.example.com/acme-hk@1.0.0#/hk.pkl"

steps {
  ["typecheck"] { check = "tsc --noEmit" }
}
```

For a package URL on any host, hk downloads `https://<host>/<path>/<name>@<version>.zip`. It does not fetch the metadata file, so the ZIP is the part hk needs, and the metadata is for the Pkl CLI. The download is kept in [`HK_PKL_CACHE_DIR`](/environment_variables#hk-pkl-cache-dir) under a name derived from that URL, so:

- A package that has been used once evaluates with no network, including with [`HK_PKL_OFFLINE=1`](/environment_variables#hk-pkl-offline) and when the host is down. On CI, persist the cache directory between runs to get the same result.
- A cached version never changes. To change the shared configuration, publish a new version and update the version in each repository’s `amends` line. Replacing a file at an existing URL does not reach machines that cached it.
- With `HK_PKL_OFFLINE=1`, a first run on a machine with no cached copy fails and names the package URL and the cache location.

The shared module above amends hk 2.4.0’s schema. A hk executable of the same version has that package built in, so it needs no download. Pinning a different hk version downloads that version’s package once, from `github.com`, unless you [mirror it](#share-rewrite).

A host with a certificate from a private certificate authority needs [`HK_PKL_CA_CERTIFICATES`](/environment_variables#hk-pkl-ca-certificates). Without it, the download fails with `error sending request`.

hk sends no credentials by itself. For a host that needs Basic authentication, see [credentials in a rewrite](#share-rewrite).

### Use relative amends or a Git submodule {#share-submodule}

Keep the shared files in a Git repository and amend them by path. As a submodule:

```sh
git submodule add https://example.com/acme/hk-shared .hk-shared
```

```pkl
amends ".hk-shared/hk.pkl"

steps {
  ["local-check"] { check = "make local-check" }
}
```

The shared files can import each other with relative paths, and each can amend or import hk’s schema by its package URL. Updating the submodule commit is the upgrade, and clones need `git submodule update --init` (or `git clone --recurse-submodules`, or `submodules: true` for `actions/checkout`) before hk can read the configuration. A path outside the repository, such as `amends "../shared-config/hk.pkl"`, works the same way, though other clones need the same layout. hk re-evaluates the configuration when any file it imports changes.

### Mirror or redirect downloads {#share-rewrite}

[`HK_PKL_HTTP_REWRITE`](/environment_variables#hk-pkl-http-rewrite) replaces the start of a URL hk would download, and it takes several rules separated by commas. Set it in the environment of the hk process, such as your shell profile or CI configuration. Use it to serve packages from an internal mirror:

```sh
export HK_PKL_HTTP_REWRITE="https://github.com/jdx/hk/releases/download/=https://mirror.example.com/hk/,https://hk-config.example.com/=https://files.internal.example/hk-config/"
```

With these rules, `package://hk-config.example.com/acme-hk@1.0.0` downloads from `https://files.internal.example/hk-config/acme-hk@1.0.0.zip`. The rewrite is applied to the ZIP URL and to plain HTTPS modules, and the longest matching prefix wins.

The target can carry Basic authentication credentials, which `pkl` itself does not support for packages: `https://user:token@files.internal.example/hk-config/`. A failed download prints the rewritten URL, credentials included, so keep that output out of shared logs and use a token with narrow, read-only access. A rule without an `=` is ignored with a warning, and a comma cannot appear inside a rule.

### Behind a proxy {#share-proxy}

Downloads follow the standard proxy variables. hk uses the first of `http_proxy`, `HTTP_PROXY`, `https_proxy`, and `HTTPS_PROXY` that is set and nonempty for every download, whatever its scheme, and honors `no_proxy` or `NO_PROXY` to skip it for some hosts:

```sh
HTTPS_PROXY=http://proxy.example.com:3128 NO_PROXY=files.internal.example hk check
```

A proxy that intercepts TLS needs its CA in [`HK_PKL_CA_CERTIFICATES`](/environment_variables#hk-pkl-ca-certificates).

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
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"

jobs = 4
fail_fast = false
skip_steps = List("optional-check")
```

For user files amending `Config.pkl`, hooks and steps merge additively with the project: user configuration adds names the project does not define, and project definitions win on collisions. Use `hk.local.pkl` to replace project behavior locally.

For removed `UserConfig.pkl` fields and legacy paths, see the
[hk v2 migration guide](/migration-v2).

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
