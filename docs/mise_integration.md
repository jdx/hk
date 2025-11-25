# mise integration

Most git-hook managers provide features that hk's sister project, [mise-en-place](https://github.com/jdx/mise), already provides. For this reason, you will want to use mise and hk together if you'd like
to use any of the features described below.

To default hk to enable these mise features, set [`HK_MISE=1`](/configuration#hk-mise).

## `hk init --mise`

Use the `--mise` flag on generate to have hk create a new `mise.toml`
file in the root of the repository that installs hk and defines a `pre-commit` task so users can run `mise run pre-commit` as a "shortcut" for `hk run pre-commit`. Of course, that's actually longer, but the advantage here is that tasks can be used consistently for all the project actions, not just git hooks.

## `hk install --mise`

Use the `--mise` flag on install to make the hook use `mise x` to execute the hooks. This will setup the mise environment (namely, add tools to PATH) for them to be used in hk.

By using `mise x`, other developers will not need to have mise already activated in their environment to use the hooks. It's useful for working
with developers who don't typically use mise but want hooks on a particular project to work with the tools defined in `mise.toml`.

## Tool Management

mise's tool management feature lets you define the version of all of the tools used in `hk.pkl` in a single place. To use, run `mise use` on
all the tools you wish to use:

```sh
mise use hk
mise use jq
mise use npm:prettier
```

This will create a `mise.toml` file that can be committed into the project. See the [mise dev tool docs](https://mise.jdx.dev/dev-tools/) for more information.

## Task Management

[mise tasks](https://mise.jdx.dev/tasks/) can be used inside hk steps
which provide a lot of functionality like dependency management, option
parsing, parallel execution, and more.

Just run mise in `hk.pkl` like any other command:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.25.0/hk@1.25.0#/Config.pkl"

`pre-commit` {
    ["prelint"] {
        check = "mise run prelint"
        exclusive = true // ensures this completes before the next steps
    }
    // ... more steps ...
}
```

## Environment Variables

You can define an `[env]` section in `mise.toml` for defining env vars to be used in the hooks:

```toml
[env]
PRETTIER_CONFIG = ".prettierrc.json"
```

mise has much more functionality around environment variables, so see the [mise docs](https://mise.jdx.dev/environments/) for more information.
