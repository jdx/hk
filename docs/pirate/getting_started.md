---
description: Bring hk aboard, chart yer first checks, and work the same steps in Git hooks and CI.
sourceHash: 05c1c9f04f24
---

# Getting under way

Rig hk on a Git repository ye already sail, then set the same linters to work when ye commit, when ye work locally, and when CI (the harbour-master) runs.

## Bringing hk aboard {#installation}

Pick one way to bring hk aboard:

::: code-group

```sh [mise]
mise use hk
```

```sh [Homebrew]
brew install hk
```

```sh [Cargo]
cargo install hk --locked
```

:::

Make sure hk came aboard and answers:

```sh
hk --version
```

Prebuilt binaries are also waiting at [GitHub releases](https://github.com/jdx/hk/releases). By default hk reads its charts with the built-in [pklr evaluator](/pkl_introduction#evaluators), so ye do not need to install the Pkl CLI.

hk's GitHub releases are immutable and carry GitHub release attestations. To check that a binary ye hauled down is exactly what was published for that release, use the [GitHub CLI](https://cli.github.com/) (2.81 or newer):

```sh
VERSION=v2.4.0 # replace with the tag of your downloaded release
gh release verify-asset "$VERSION" hk-x86_64-unknown-linux-gnu.tar.gz --repo jdx/hk
```

`gh release verify "$VERSION" --repo jdx/hk` checks the release itself. Verification confirms that the file came from the release; it does not review what the release contains.

## Rigging the ship {#project-setup}

From the root of yer repository, have hk draw up a configuration:

```sh
hk init
```

hk spots yer tools from the project's files and writes `hk.pkl`, the ship's charts. Read over its steps before ye run them. To pick the tools and hooks yerself, use `hk init --interactive`. hk searches recursively, through every directory, for the source files that give a tool away, honouring ignore rules and never following symlinks; `.gitignore` applies inside Git repositories, and `.ignore` works outside Git too. .NET manifest globs and configuration indicators are only looked for at the root, so a nested workspace is not activated behind yer back.

When `hk init --mise` is used, hk merges into an existing `mise.toml` only the entries it lacks: `hk` if it's missing, and `pre-commit` when there's none. The quartermaster's existing pins, comments, tools, and tasks are all kept as ye left them. `--force` governs `hk.pkl` and does not reset `mise.toml`. hk inspects only literal local task includes; an included flat `pre-commit` task stops it adding a duplicate. An include that is unknown, remote, dynamic, missing, unreadable, or malformed stops the insertion, and hk sings out a warning.

::: tip Muster yer lookouts
Builtins, the standing crew, configure commands; they do not install the tools those commands invoke. Bring the linters ye chose aboard with yer project's package manager or [mise](/mise_integration), and make sure hk can find them on `PATH`.
:::

## Yer first charts {#your-first-configuration}

This complete example needs no extra tools. The `trailing_whitespace` and `newlines` builtins, the standing crew, run hk's own `hk util` commands, so ye can try hk before bringing a single linter aboard. The two sailmakers haul at once, and a lashing on each file keeps them from colliding on the same cargo.

Replace the whole of the `hk.pkl` that `hk init` drew up with this example. If `hk init` spotted linters, its steps need those tools aboard on `PATH`; the example below does not.

```pkl
amends "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v2.4.0/hk@2.4.0#/Builtins.pkl"

steps {
  ["trailing_whitespace"] = Builtins.trailing_whitespace
  ["newlines"] = Builtins.newlines
}
```

The `amends` line loads hk's configuration schema. `Builtins` supplies reusable step definitions: the standing crew. Top-level `steps` is the recommended place to start: from it hk creates `check`, `fix`, and `pre-commit` hooks that share these steps, so all three muster the self-same crew.

With these charts, `pre-commit` fixes the staged files while yer unstaged work is stowed in the hold. `check` inspects yer working tree, and `fix` applies fixes to it. A step whose file patterns match none of the selected files is skipped.

When ye're ready for lookouts that know yer language, sign on more hands by adding entries to the existing `steps` block; a second top-level `steps` block would keep the charts from loading. These builtins call on outside tools, so bring Prettier, ESLint, and Ruff aboard and configure them first, or swap them for [builtins](/builtins) that suit yer project:

```pkl
steps {
  ["trailing_whitespace"] = Builtins.trailing_whitespace
  ["newlines"] = Builtins.newlines
  ["prettier"] = Builtins.prettier
  ["eslint"] = Builtins.eslint
  ["ruff"] = Builtins.ruff
}
```

Top-level `steps` is optional, matey. Ye can instead define steps only inside explicit `hooks`, giving each pipe its own hands, or use explicit hooks to customize the shared setup. See [hook defaults](/configuration#hook-defaults).

Make sure the charts are sound without sending any linters aloft:

```sh
hk validate
```

## Checking the cargo and mending the canvas {#checking-and-fixing-code}

```sh
hk check             # Check modified files
hk fix               # Apply available fixes
hk check --all       # Check all files, useful for CI
hk check src/main.ts # Check a specific file
hk check --step newlines
```

With the charts above, the modified files include the staged, unstaged, and untracked ones: cargo loaded aboard, cargo left on the dock, and cargo Git doesn't track yet. `--all` selects the tracked files plus eligible untracked ones; ignore rules and exclusions still apply. Hook settings and flags can change which files are selected.

Check commands should be read-only: lookouts look, they don't touch. Fix commands may edit files, and some findings need mending by hand. `hk fix` leaves its fixes unstaged by default; use `hk fix --stage` to stage them. The default `pre-commit` hook stages its fixes. Review `git diff` and `git diff --cached` to see what the sailmakers changed.

## Read the passage plan {#preview-a-run}

Use the passage plan to see which steps and files hk selects:

```sh
hk check --plan
hk check --why newlines
hk check --all --plan --json
```

These commands do not execute the hook's steps; no hand goes aloft. See [troubleshooting](/logging) if a step is missing or behaves strangely.

## Rig the hooks {#install-hooks}

Choose the scope that fits how ye sail:

| Scope                                         | Command               | What it does                                                                      |
| --------------------------------------------- | --------------------- | --------------------------------------------------------------------------------- |
| The whole fleet (all repositories), Git 2.54+ | `hk install --global` | Rig it once in yer user Git config; ships without an hk configuration are skipped |
| This ship (the current repository)            | `hk install`          | Rig the hooks this project defines; works with older Git versions too             |

On Git 2.54+, hk uses Git's configuration-based hooks. On older Git, rigging a single ship (a per-repository install) writes script shims. Use `hk install --legacy` to ask for shims outright.

If hk is already rigged across the fleet (installed globally), `hk install` skips the local installation and clears away stale local hk hooks. `--force-local` overrides that, but combining local and global hooks can cause duplicate runs: the pipe may call all hands twice over.

::: tip The quartermaster's tools in Git hooks
On Git 2.54+, the recommended course is `hk install --global --mise`, which launches hooks through `mise x`. The installer writes down where mise lives, so mise must be on `PATH` while ye install, but Git does not need it on its runtime `PATH`. For an installation scoped to one repository, on any supported Git version, use `hk install --mise`; this local launcher does need mise on Git's runtime `PATH`.
:::

Commit `hk.pkl` so all yer shipmates sail by the same charts. Installing the hooks is local to each developer's machine or clone.

To take an installation down, use `hk uninstall` or `hk uninstall --global`. The [install reference](/cli/install) lists every option.

## Calling all hands: running hooks {#running-hooks}

Once the hooks are rigged, Git sounds the pipe for each configured hook by itself. Ye can also sound it directly:

```sh
hk run pre-commit
```

A hook run by hand does everything the hook is configured to do: fixes, staging, and stashing included. To inspect it first, use `hk run pre-commit --plan`.

## Where to sail next {#next-steps}

- [Git hooks and stowing the hold](/hooks): take command of automatic fixes and partial commits.
- [Continuous integration, the harbour-master](/ci): check a full repository or a branch.
- [Ships in bottles, the configuration examples](/reference/examples/): start from a JavaScript, Python, or monorepo setup.
- [The ship's charts, configuration](/configuration): customize steps, profiles (the watches), and local overrides.

Heave away, haul away, and ye're bound away for the main!
