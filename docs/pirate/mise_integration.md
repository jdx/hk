---
description: Let mise, the ship's quartermaster, provision hk and yer linter versions, environment variables, and the running of Git hooks.
sourceHash: 615e042a7cef
---

# mise integration: the quartermaster

[mise](https://mise.jdx.dev/) be the ship's quartermaster: it manages tools, environments, and tasks. hk picks out the cargo (the files) and coordinates the checks and fixes. Together they let a whole crew share the same tool versions and run hooks from terminals, editors, and CI alike.

mise is optional, mind: hk can run any executable it finds on `PATH`.

## Provision the tools {#install-tools}

From yer project directory, there's but three words left to say, and a sailmaker to sign aboard besides:

```sh
mise use hk
mise use npm:prettier
```

Commit the `mise.toml` that results, so it sails with the ship. Every other sailor aboard can then run `mise install` to install the very versions it declares.

If a language package manager already looks after a tool, keep it there, matey. For example, to put a Node project's installed executables within reach through mise:

```toml
[env]
_.path = ["node_modules/.bin"]
```

Run the project's package installation command before ye call on hk. See [mise tool management](https://mise.jdx.dev/dev-tools/) for the backends the quartermaster supports.

## Put the tools within Git's reach {#make-tools-available-to-git}

On Git 2.54+, install hk's hooks, the bosun's pipes, once per developer machine, with mise integration:

```sh
hk install --global --mise
```

The global launcher be the recommended rig. It uses `mise x` to provision each project's environment before running hk, and any repository without an hk configuration is skipped. mise must be on `PATH` while ye install; the global launcher writes down where mise's executable lives, so Git need not find it when the hook runs.

To rig one ship at a time instead (a repository-scoped setup, on any supported Git version), install the hooks separately in each repository:

```sh
hk install --mise
```

The local launcher uses `mise x` too, but it needs Git to find mise on its `PATH` at the moment the hook runs. With either launcher, no sailor needs an activated shell.

Use one installation scope at a time. When moving a ship from a local installation to the recommended global one, strike the local hooks first:

```sh
hk uninstall
hk install --global --mise
```

Setting `HK_MISE=1` in yer standing orders makes `--mise` the default for later `hk init` and `hk install` commands. It does not rewrite a launcher that's already installed; that waits until installation runs again.

## Draw up a starter setup {#generate-a-starter-setup}

`hk init --mise` draws up `hk.pkl` (the ship's charts) and, if there isn't one already, a `mise.toml` with hk configured and a `pre-commit` task.

Run `hk init --mise`, then, on Git 2.54+, install the recommended global launcher:

```sh
hk init --mise
hk install --global --mise
```

On an older Git, or to rig just this one repository, use `hk install --mise` instead.

Look over the tools and tasks it drew up. A `mise.toml` that's already aboard is kept as it is.

## Call a mise task from a hand {#call-a-mise-task-from-a-step}

Use a task when a check earns its keep outside Git hooks too:

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test"] {
        check = "mise run test"
      }
    }
  }
}
```

A step with no `glob` runs whether any files match or not. Add a pattern if the task should only turn out for certain file types.

If a task writes files outside the paths its step was handed, it slips its lashings: declare suitable dependencies, or use `exclusive = true`, to keep it from fouling the other steps.

## Share the standing orders (environment variables) {#share-environment-variables}

Use mise's `[env]` section for the standing orders that yer project's commands should follow:

```toml
[env]
NODE_ENV = "development"
```

For orders meant only for the linter commands, use hk's global, hook, or step `env` blocks. See [mise environments](https://mise.jdx.dev/environments/) and [hk's charts, the configuration](/configuration).

## Sail under the harbour-master's eye (CI) {#run-in-ci}

Once mise is aboard:

```sh
mise install
mise exec -- hk check --all
```

If the steps use language package dependencies, install those too. See [the harbour-master, continuous integration](/ci) for branch comparisons, profiles, and diagnostics.

## Every directory its own provisions (monorepos) {#per-directory-environments-monorepos}

When `HK_MISE=1` is set, hk asks the quartermaster for the mise environment of each step's `dir`, by running `mise env` in that directory. It asks once per directory per run, and caches the answer. Tools and env vars that a subdirectory's mise config defines (such as the `mise.toml` at a [mise monorepo](https://mise.jdx.dev/tasks/monorepo.html) config root) are within reach of the steps working in that directory, even when hk sets out from the repo root:

```pkl
hooks {
    ["check"] {
        steps {
            ["oxlint"] = (Builtins.ox_lint) {
                // with HK_MISE=1, tools from subproject/mise.toml are on PATH
                dir = "subproject"
            }
        }
    }
}
```

Explicit step `env` values always win over the environment mise provides: a hand's own orders outrank the quartermaster's.
