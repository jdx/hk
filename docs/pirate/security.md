---
description: What hk sets running when ye call on it, what the chart-maker's language can read and fetch, how a whole-fleet hook reaches every ship, and what --safe does and does not promise.
sourceHash: 97b8817d2b92
---

# Security model: the ship's defences

hk runs the commands the ship's charts declare. Treat `hk.pkl` like a Makefile or a harbour-master's workflow: 'tis code, and running hk in a ship runs that ship's charts with yer own privileges, matey.

hk has no trust prompt and no sandbox. This page tells plainly what it does, so ye can decide what to review and where to sail it.

## What counts as code {#what-counts-as-code}

hk executes or evaluates all of these when it loads the charts:

- **The project charts.** hk looks in the current directory, then in each parent directory, for the first directory that has `hk.local.pkl`, `.config/hk.local.pkl`, `hk.pkl`, or `.config/hk.pkl`. `HK_FILE` replaces that search with one path.
- **Everything the charts import or amend.** That includes local files, `https://` modules, and `package://` packages, such as the one that provides `Builtins` and `Config`.
- **Yer sea chest**, `~/.config/hk/config.pkl` (or `$HK_CONFIG_DIR/config.pkl`). Its hands and hooks are added to every ship's charts. When a ship defines the same hand, the ship's definition wins.

A hand's `check`, `check_list_files`, `check_diff`, and `fix` commands, its `shell` and `prefix`, and a hook's `report` command then run on yer own machine as ye, with yer own standing orders. A hand's `env` and a master chart's `env` are added to those orders. hk exports the master chart's `env` to its own process, so every hand and `report` command inherits it.

A ship ye clone does not carry the hooks rigged with `hk install`, because Git does not clone `.git/hooks` or a ship's local config. It does carry its `hk.pkl`, which runs as soon as anything runs hk there.

## What Pkl evaluation can do {#pkl-evaluation}

hk evaluates Pkl with its built-in evaluator, not the `pkl` binary. Evaluation happens before any hand runs, and every call that loads the project charts does it, including `hk validate`, `hk install`, `hk check`, and the hooks Git runs. Evaluation can:

- **Read standing orders** (environment variables) with `read("env:NAME")`. hk records the variables the charts read and includes their values in its locker's cache key.
- **Read local files** with `read("file:///path")` or a relative path. Nothing restricts the path to the ship's own directory.
- **Send a boat ashore.** The charts can `import` or `amends` an `https://` module, `read` an `https://` URL, or import a `package://` package. Packages are downloaded to `HK_PKL_CACHE_DIR`.

The evaluator has no facility for running a process. Commands run later, when the hands run.

hk does not pin or verify a checksum for a downloaded module or package. The version in the URL, as in `https://github.com/jdx/hk/releases/download/vX.Y.Z/hk@X.Y.Z.zip`, is the only pin. The only check is that a downloaded package is a readable archive.

Three settings change where evaluation gets its input:

| Setting               | Effect                                                                                                                                                                                                                                                                                                                                           |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `HK_PKL_EMBEDDED`     | On by default. When the charts import the package for the running hk's own version, hk uses the copy embedded in the binary and sends no boat ashore. See [environment variables](/environment_variables#hk-pkl-embedded).                                                                                                                       |
| `HK_PKL_OFFLINE`      | Disables network access. Packages already in the locker, and the embedded package, still load; anything else fails. See [environment variables](/environment_variables#hk-pkl-offline).                                                                                                                                                         |
| `HK_PKL_HTTP_REWRITE` | Rewrites URL prefixes before the evaluator fetches them, in the form `https://source.example/=https://mirror.example/`. It applies to modules and packages alike, so whoever sets it controls what those URLs return. It is part of the charts' cache key. See [environment variables](/environment_variables#hk-pkl-http-rewrite). |

`HK_PKL_HTTP_REWRITE` is read from the standing orders of the hk process. The charts cannot set it, but anything that can set the process environment can use it to point imports at another server.

## Global hooks {#global-hooks}

`hk install --global` (Git 2.54 or newer) writes `hook.hk-<event>.command` and `hook.hk-<event>.event` entries to `~/.gitconfig`. Run inside a ship with charts, it rigs that ship's enabled hooks other than `check` and `fix`; run anywhere else, it rigs `commit-msg`, `pre-commit`, `pre-push`, and `prepare-commit-msg`. Each hook runs `hk run <event> --from-hook` using the absolute path of the hk that rigged it.

Git then runs hk for those events in every ship on the machine, the whole fleet:

- With no project charts in the ship or any parent directory, hk exits silently. It does not load yer sea chest.
- With project charts, hk evaluates them and runs their hands. This includes a ship ye have just cloned, before ye have read its `hk.pkl`, and a ship where ye never ran `hk install`.

A global install changes who decides that a ship's `hk.pkl` runs on a voyage: without it, ye do, by running `hk install` in the ship; with it, the ship does.

To opt out, set `HK=0` for one command (`HK=0 git commit`), or stand down a global entry in one ship with `git config --local hook.hk-pre-commit.enabled false`. Use the entry's event name for the other hooks.

## What `--safe` does {#safe}

`--safe` is a gate on declared effects. Before any hand starts, hk looks at every command the run could execute: the `check`, `check_list_files`, `check_diff`, or `fix` command for the run, plus the fallback and recheck commands of a check-first hand. Hands whose jobs are all skipped are not examined. hk refuses the whole run if any of those commands has an effect that is unknown or `destructive`:

```text
Error: --safe refused to run:
  probe.check: effect is unknown
```

A command has a known effect only when `hk.pkl` wraps it in `Config.CommandSpec` with `effect = "read"`, `"write"`, or `"destructive"`. A plain string, `Script`, or `Command` has an unknown effect. Most builtins declare effects. See [coding agents](/agents) for how `--safe` fits the clockwork hands' workflow; the MCP server's `start_safe_check` and `start_safe_fix` tools pass it, and `start_check` does not.

`--safe` is not a sandbox:

- The effect is a declaration written in the same `hk.pkl` that defines the command. hk does not observe what a command does. A command declared `read` can still write files, and one declared `write` can still reach the network.
- It does not limit the filesystem, network, or environment a command sees.
- It does not examine a hook's `report` command, which runs after the hands.
- It does not restrict Pkl evaluation. The reads and fetches described above happen before the gate runs.

So `--safe` helps keep a clockwork hand or automation from running commands yer charts do not vouch for. It does not make untrusted charts safe to run.

## What hk does not do {#limits}

- It does not ask before running a ship's `hk.pkl`, and has no trust or allowlist mechanism for charts.
- It does not sandbox hands, `report` commands, or Pkl evaluation.
- It does not verify checksums or signatures for modules and packages it downloads.
- It does not check that a command's declared effect matches what it does.

## Reporting a vulnerability {#reporting}

See [SECURITY.md](https://github.com/jdx/hk/blob/main/SECURITY.md).
