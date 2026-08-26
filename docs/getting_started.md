# Getting Started

This guide takes you from installing hk to running your first checks. Most projects can be set up
in a few minutes.

## 1. Install hk

From the project you want to configure, install hk with [mise](https://mise.jdx.dev/):

```sh
cd my-project
mise use hk
hk --version
```

Other installation methods:

- [Homebrew](https://formulae.brew.sh/formula/hk): `brew install hk`
- Cargo: `cargo install hk`
- [Aqua](https://github.com/aquaproj/aqua-registry/blob/main/pkgs/jdx/hk/registry.yaml): `aqua g -i jdx/hk`

hk uses its built-in Pkl evaluator by default, so you do not need to install the Pkl CLI. See the
[Pkl introduction](/pkl_introduction) if you want to use the standalone evaluator or learn the
configuration language.

## 2. Enable git hooks

With Git 2.54 or newer, install hk once for all repositories on your machine:

```sh
hk install --global
```

The global hooks are a silent no-op in repositories without an `hk.pkl`, so enabling them does not
require every repository to use hk.

On older Git versions, or when you only want hk in one repository, skip this step for now. You will
install the repository hooks after creating `hk.pkl` in the next step.

::: warning Avoid duplicate hooks
Do not combine `hk install --global` with a per-repository install unless you intentionally want both.
Git combines hook commands from its global and local configuration, which can cause hk to run twice.
:::

## 3. Create a project configuration

From the project root, run:

```sh
hk init
```

hk detects common project files and creates an `hk.pkl` using the relevant
[built-in linters](/builtins). To review and select linters and hooks yourself, use:

```sh
hk init --interactive
```

Builtins define how hk invokes a tool; they do not install the tool itself. Each selected linter or
formatter must be available on `PATH`. If you use mise, the [mise integration](/mise_integration)
can manage those tool versions with the project.

If you skipped the global installation, install the hooks defined by the new configuration now:

```sh
hk install
```

Per-repository installation uses Git's config-based hooks on Git 2.54 or newer and `.git/hooks/`
scripts on older versions. See [`hk install`](/cli/install) for all installation modes, including
`--legacy` and `--mise`.

## 4. Review `hk.pkl`

A typical configuration shares the same linters between automatic hooks and manual commands:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.56.1/hk@1.56.1#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.56.1/hk@1.56.1#/Builtins.pkl"

local linters = new Mapping {
  ["eslint"] = Builtins.eslint
  ["prettier"] = Builtins.prettier
}

hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = linters
  }
  ["check"] {
    steps = linters
  }
  ["fix"] {
    fix = true
    steps = linters
  }
}
```

In this example, `pre-commit` and `hk fix` may modify files. During a commit, `stash = "git"`
protects unstaged changes while fixes are applied to staged content.

See the [configuration guide](/configuration) for custom commands, dependencies, profiles, groups,
and monorepo support. The [configuration examples](/reference/examples/) include complete JavaScript,
Python, custom-linter, and monorepo setups.

## 5. Test the setup

Run checks without making changes:

```sh
hk check --all
```

Apply available fixes:

```sh
hk fix --all
```

To test the pre-commit configuration independently of its Git installation:

```sh
hk run pre-commit
```

`pre-commit` selects staged files by default. Running it without `--staged` also preserves the
hook's configured stashing behavior for partially staged files. This validates the configured
steps; make a test commit when you need to verify the installed Git hook end to end.

By default, `hk check` and `hk fix` only operate on modified files. Use `--all` in CI or when you
want to verify the entire repository, and `--from-ref main` to check files changed since a branch or
commit.

## Common next steps

- Browse and customize [built-in linters](/builtins).
- Learn how hk selects files and schedules steps in [Hooks](/hooks).
- Add hk to a mise-managed project with [`hk init --mise`](/mise_integration#hk-init-mise).
- Configure a shared user-level `~/.config/hk/config.pkl` in [hkrc](/configuration#hkrc).
- Diagnose a configuration with [`hk config explain`](/cli/config/explain) and
  [`hk config sources`](/cli/config/sources).

## Removing hooks

Remove the installation using the same scope you used to install it:

```sh
hk uninstall --global # global installation
hk uninstall          # current repository
```

## Manual Git configuration

If you prefer not to run `hk install --global`, you can add hooks directly to `~/.gitconfig`:

```ini
[hook "hk-pre-commit"]
    command = test "${HK:-1}" = "0" || hk run pre-commit --from-hook "$@"
    event = pre-commit
[hook "hk-pre-push"]
    command = test "${HK:-1}" = "0" || hk run pre-push --from-hook "$@"
    event = pre-push
[hook "hk-commit-msg"]
    command = test "${HK:-1}" = "0" || hk run commit-msg --from-hook "$@"
    event = commit-msg
```

`--from-hook` makes repositories without a matching hk configuration exit silently. The `HK`
check provides a per-command escape hatch: use `HK=0 git commit` to bypass hk temporarily. If hk is
installed through mise but is not automatically activated, replace `hk` with `mise x -- hk`.

To disable one globally configured event for a particular repository:

```sh
git config --local hook.hk-pre-commit.enabled false
```

See the complete [`hk install` reference](/cli/install) for additional flags and installation
behavior.
