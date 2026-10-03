---
description: What running hk executes, what Pkl evaluation can read and fetch, how global hooks reach every repository, and what --safe does and does not guarantee.
---

# Security model

hk runs the commands your configuration declares. Treat `hk.pkl` like a Makefile or a CI workflow: it is code, and running hk in a repository runs that repository's configuration with your privileges.

hk has no trust prompt and no sandbox. This page states what it does so you can decide what to review and where to run it.

## What counts as code {#what-counts-as-code}

hk executes or evaluates all of the following when it loads configuration:

- **The project config.** hk looks in the current directory, then in each parent directory, for the first directory that has `hk.local.pkl`, `.config/hk.local.pkl`, `hk.pkl`, or `.config/hk.pkl`. `HK_FILE` replaces that search with one path.
- **Everything the config imports or amends.** That includes local files, `https://` modules, and `package://` packages such as the one that provides `Builtins` and `Config`.
- **The user config**, `~/.config/hk/config.pkl` (or `$HK_CONFIG_DIR/config.pkl`). Its steps and hooks are added to every project's configuration. When a project defines the same step, the project's definition wins.

A step's `check`, `check_list_files`, `check_diff`, and `fix` commands, its `shell` and `prefix`, and a hook's `report` command then run on your machine as you, with your environment. A step's `env` and a root config's `env` are added to that environment. hk exports the root config's `env` to its own process, so every step and `report` command inherits it.

A repository you clone does not carry hooks installed with `hk install`, because Git does not clone `.git/hooks` or a repository's local config. It does carry its `hk.pkl`, which runs as soon as anything runs hk there.

## What Pkl evaluation can do {#pkl-evaluation}

hk evaluates Pkl with its built-in evaluator, not the `pkl` binary. Evaluation happens before any step runs, and every command that loads the project config does it, including `hk validate`, `hk install`, `hk check`, and the hooks Git runs. Evaluation can:

- **Read environment variables** with `read("env:NAME")`. hk records the variables a config reads and includes their values in its config cache key.
- **Read local files** with `read("file:///path")` or a relative path. Nothing restricts the path to the project directory.
- **Fetch over the network.** A config can `import` or `amends` an `https://` module, `read` an `https://` URL, or import a `package://` package. Packages are downloaded to `HK_PKL_CACHE_DIR`.

The evaluator has no facility for running a process. Commands run later, when steps run.

hk does not pin or verify a checksum for a downloaded module or package. The version in the URL, as in `https://github.com/jdx/hk/releases/download/vX.Y.Z/hk@X.Y.Z.zip`, is the only pin. The only check is that a downloaded package is a readable archive.

Three settings change where evaluation gets its input:

| Setting                | Effect                                                                                                                                                                         |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `HK_PKL_EMBEDDED`      | On by default. When a config imports the package for the running hk's own version, hk uses the copy embedded in the binary and makes no request. See [environment variables](/environment_variables#hk-pkl-embedded). |
| `HK_PKL_OFFLINE`       | Disables network access. Packages already in the cache, and the embedded package, still load; anything else fails. See [environment variables](/environment_variables#hk-pkl-offline). |
| `HK_PKL_HTTP_REWRITE`  | Rewrites URL prefixes before the evaluator fetches them, in the form `https://source.example/=https://mirror.example/`. It applies to modules and packages alike, so whoever sets it controls what those URLs return. It is part of the config cache key. See [environment variables](/environment_variables#hk-pkl-http-rewrite). |

`HK_PKL_HTTP_REWRITE` is read from the environment of the hk process. A config cannot set it, but anything that can set the process environment can use it to point imports at another server.

## Global hooks {#global-hooks}

`hk install --global` (Git 2.54 or newer) writes `hook.hk-<event>.command` and `hook.hk-<event>.event` entries to `~/.gitconfig`. Run inside a project with a config, it installs that project's enabled hooks other than `check` and `fix`; run anywhere else, it installs `commit-msg`, `pre-commit`, `pre-push`, and `prepare-commit-msg`. Each hook runs `hk run <event> --from-hook` using the absolute path of the hk that installed it.

Git then runs hk for those events in every repository on the machine:

- With no project config in the repository or any parent directory, hk exits silently. It does not load the user config.
- With a project config, hk evaluates it and runs its steps. This includes a repository you have just cloned, before you have read its `hk.pkl`, and a repository where you never ran `hk install`.

A global install changes who decides that a repository's `hk.pkl` runs on commit: without it, you do, by running `hk install` in the repository; with it, the repository does.

To opt out, set `HK=0` for one command (`HK=0 git commit`), or disable a global entry in one repository with `git config --local hook.hk-pre-commit.enabled false`. Use the entry's event name for the other hooks.

## What `--safe` does {#safe}

`--safe` is a gate on declared effects. Before any step starts, hk looks at every command the run could execute: the `check`, `check_list_files`, `check_diff`, or `fix` command for the run, plus the fallback and recheck commands of a check-first step. Steps whose jobs are all skipped are not examined. hk refuses the whole run if any of those commands has an effect that is unknown or `destructive`:

```text
Error: --safe refused to run:
  probe.check: effect is unknown
```

A command has a known effect only when `hk.pkl` wraps it in `Config.CommandSpec` with `effect = "read"`, `"write"`, or `"destructive"`. A plain string, `Script`, or `Command` has an unknown effect. Most builtins declare effects. See [coding agents](/agents) for how `--safe` fits an agent workflow; the MCP server's `start_safe_check` and `start_safe_fix` tools pass it, and `start_check` does not.

`--safe` is not a sandbox:

- The effect is a declaration written in the same `hk.pkl` that defines the command. hk does not observe what a command does. A command declared `read` can still write files, and one declared `write` can still reach the network.
- It does not limit the filesystem, network, or environment a command sees.
- It does not examine a hook's `report` command, which runs after the steps.
- It does not restrict Pkl evaluation. The reads and fetches described above happen before the gate runs.

So `--safe` helps keep an agent or automation from running commands your configuration does not vouch for. It does not make an untrusted configuration safe to run.

## What hk does not do {#limits}

- It does not ask before running a repository's `hk.pkl`, and has no trust or allowlist mechanism for configs.
- It does not sandbox steps, `report` commands, or Pkl evaluation.
- It does not verify checksums or signatures for modules and packages it downloads.
- It does not check that a command's declared effect matches what it does.

## Reporting a vulnerability {#reporting}

See [SECURITY.md](https://github.com/jdx/hk/blob/main/SECURITY.md).
