# mise integration

Many git hook managers build in features that hk's sister project, [mise-en-place](https://github.com/jdx/mise), already provides. hk leaves those to mise, so use mise and hk together if you want
any of the features described below.

To default hk to enable these mise features, set [`HK_MISE=1`](/environment_variables#hk-mise).

:::info
Setting `HK_MISE=1` will wrap your Git hooks with `mise x`. This ensures that mise automatically sets up the correct environment and tool versions before running hk, even if other developers haven't activated mise in their shell.
:::

## `hk init --mise`

Use the `--mise` flag on `hk init` to have hk create a new `mise.toml`
file in the root of the repository that installs hk and defines a `pre-commit` task, so users can run `mise run pre-commit` as a "shortcut" for `hk run pre-commit`. Of course, that's actually longer, but the advantage is that tasks can be used consistently for all project actions, not just git hooks.

## `hk install --mise`

Use the `--mise` flag on `hk install` to make the installed hooks run hk through `mise x`. This sets up the mise environment (namely, adding tools to `PATH`) before hk runs.

With `mise x`, other developers do not need mise activated in their shell to use the hooks. It's useful for working
with developers who don't typically use mise but want hooks on a particular project to work with the tools defined in `mise.toml`.

## Tool Management

mise's tool management lets you pin the versions of all the tools used in `hk.pkl` in a single place. Run `mise use` for
each tool you want to manage:

```sh
mise use hk
mise use jq
mise use npm:prettier
```

This will create a `mise.toml` file that can be committed into the project. See the [mise dev tool docs](https://mise.jdx.dev/dev-tools/) for more information.

## Task Management

[mise tasks](https://mise.jdx.dev/tasks/) can be used inside hk steps.
They provide dependency management, option parsing, parallel execution, and more.

Run `mise run` in `hk.pkl` like any other command:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.58.1/hk@1.58.1#/Config.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["prelint"] {
                check = "mise run prelint"
                exclusive = true // ensures this completes before the next steps
            }
            // ... more steps ...
        }
    }
}
```

## Environment Variables

You can define an `[env]` section in `mise.toml` to set environment variables for the hooks:

```toml
[env]
PRETTIER_CONFIG = ".prettierrc.json"
```

mise has much more functionality around environment variables, so see the [mise docs](https://mise.jdx.dev/environments/) for more information.

## Per-directory environments (monorepos)

When `HK_MISE=1` is set, hk resolves the mise environment for each step's `dir` by
running `mise env` in that directory (cached, once per directory per run). Tools and
env vars defined by a subdirectory's mise config — such as a
[mise monorepo](https://mise.jdx.dev/tasks/monorepo.html) config root's `mise.toml` —
are available to steps running in that directory, even when hk is started from the
repo root:

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

Explicit step `env` values always win over the mise-provided environment.

## Recommended Setup

The recommended approach is to use `mise.toml` as the source of truth for your tools and environment, while using `hk` specifically for managing the Git lifecycle.

By setting `HK_MISE=1` and using a `postinstall` hook, you can automate hook installation for your entire team:

```toml
[tools]
hk = "latest"
# ... other tools like prettier, actionlint, etc.

[env]
HK_MISE = 1

[hooks]
# Automatically install/update hooks when tools are installed
postinstall = "hk install --mise"
```

If hk is already configured globally (e.g. `hk install --global` from a
dotfiles setup), `hk install` automatically skips the per-repo install
and cleans up any stale local hooks, so it's safe to leave the
`postinstall` line in place across machines with mixed setups.
