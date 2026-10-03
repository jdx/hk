---
description: Set up hk for development, run focused checks, edit generated documentation, and prepare a contribution for review.
---

# Contributing

::: danger AI replies to Discussions and Issues are restricted
AI replies to Discussions and Issues are restricted, and breaking the rule is **an instant ban across all of jdx's projects**.

You may only use AI to reply to a [Discussion](https://github.com/jdx/hk/discussions) or [Issue](https://github.com/jdx/hk/issues) if you
created it, you opened a PR that fixes it, or you have already had a contribution, attributed to your GitHub account, merged into the default branch of hk. Everyone else is not allowed to use AI to reply. This is a growing problem.

This includes raw, lightly edited, reviewed, and disclosed model output. Adding an "AI-assisted"
footer does not make an AI reply acceptable on its own. If you are running an agent, make sure it
does not post to threads you are not allowed to reply to, and never let it sweep through many
threads at once.
:::

Using AI to help write and file your own Discussion or Issue is fine. Review it before posting, and
disclose that AI contributed. If you are allowed to use AI to reply, review and verify the reply before
posting it, and disclose that AI contributed.

Bug fixes, documentation improvements, and builtin definitions are welcome. For a substantial feature or behavior change, discuss the direction before investing in implementation.

## Review expectations

Open a [discussion](https://github.com/jdx/hk/discussions) or ask in [Discord](https://discord.gg/UBa7pJUN7Z) before starting a change whose scope or design is not obvious. hk has a deliberate scope; the maintainer may decline features that add complexity or long-term maintenance without a clear fit.

Before requesting review, make sure CI passes and address automated review comments. Explain the problem, resulting behavior, and validation in the PR. Contributions should be complete enough to assess without extensive coaching.

The maintainer handles a high volume of contributions across projects. Feedback or rejection may be brief, and uncertain fit can be enough to decline a change.

## Development setup

Install [mise](https://mise.jdx.dev/) and a Rust toolchain compatible with the repository’s `Cargo.toml`, then:

```sh
git clone https://github.com/jdx/hk.git
cd hk
mise install
mise run build
```

The build task generates the builtin registry before compiling hk. Development tasks put the local debug binary on `PATH`.

## Run focused checks

| Task                   | Command                              |
| ---------------------- | ------------------------------------ |
| Build                  | `mise run build`                     |
| Rust tests             | `mise run test:cargo`                |
| One Rust test          | `cargo test test_name`               |
| Bats integration tests | `mise run test:bats`                 |
| One Bats file          | `mise run test:bats test/check.bats` |
| Full test suite        | `mise run test`                      |
| Lint                   | `hk check --all`                     |
| Lint including Clippy  | `hk check --all --slow`              |
| Apply formatting fixes | `hk fix --all`                       |

`mise run test:bats` accepts several files or directories (`mise run test:bats test/check.bats test/fix.bats`). It needs `bats`, ripgrep (`rg`) and GNU `parallel` on `PATH`, and it fails when a path does not exist, matches no `test/*.bats` file, or when no test runs.

Run checks appropriate to the change. Integration tests use isolated temporary repositories and exercise Git backends. See the [test-suite guide](https://github.com/jdx/hk/blob/main/test/README.md) for fixtures and cache behavior.

## Add a builtin

1. Add `pkl/builtins/<name>.pkl` with metadata, file patterns, and commands.
2. Define Pkl-level tests in the step’s `tests` field. Use `TestMaker` from `pkl/builtins/test/helpers.pkl` for standard check/fix patterns.
3. Add a `mise tool-stub` script in `test/builtin_tool_stubs/` if the tool is not already available.
4. Regenerate and build with `mise run build`.
5. Run `mise run test:bats test/builtins_tests.bats`, or use `hk test --step <name>` with a configuration that loads the builtin.

Tests should verify meaningful behavior: a clean check, a failing check, and the expected result of a fix when supported. A builtin with `check_diff` also needs a diff test (`TestMaker.diffPass` or `diffFail`), which applies the patch `check_diff` prints the way `hk fix` does and fails if hk cannot apply it. Fix tests can't catch that, because they run `fix` directly, and `test/builtins_tests.bats` fails for a tested builtin without one. Avoid enabling batching or bypassing locks without confirming the tool’s behavior.

## Edit documentation

The website uses VitePress. Run these from the repository root:

```sh
mise run docs        # Generate reference content and start the dev server
mise run docs:build  # Generate and build the production site
```

Use the source of truth for each type of page:

| Content                | Edit here                                                                     |
| ---------------------- | ----------------------------------------------------------------------------- |
| README and guides      | `README.md`, `docs/*.md`                                                      |
| Landing page and theme | `docs/.vitepress/theme/`                                                      |
| Navigation             | `docs/.vitepress/config.mts`                                                  |
| Schema reference       | Documentation comments in `pkl/Config.pkl`                                    |
| Settings reference     | `docs` strings in `settings.toml`                                             |
| Builtin catalogue      | Metadata and definitions in `pkl/builtins/`                                   |
| CLI reference          | Rust CLI help and usage definitions; examples in `scripts/enrich-cli-docs.py` |
| Downloadable examples  | `docs/public/*.pkl`, included directly by their guide pages                   |

`mise run docs:gen` generates `docs/gen/`. To regenerate the CLI reference, use `mise run render:usage`; that task also stages its generated outputs. Run `scripts/generate-examples.sh` to validate the downloadable examples and their documentation includes.

Keep examples complete when they are intended to be copied. Label fragments, list external tool requirements, and use one package version for both schema and builtin imports. Verify internal links and inspect affected layouts at narrow and wide widths.

## Commit messages and pull requests

Use Conventional Commits with a lowercase, imperative description:

- `fix(step): handle missing files`
- `feat(builtins): add a formatter`
- `docs: clarify hook installation`
- `chore: update CI tooling`

Use `fix` for changes to CLI behavior; use `chore` for CI and infrastructure. Add a command or subsystem scope when applicable.

Open a PR ready for review with focused changes and a concise validation summary. Follow the repository’s [agent guidelines](https://github.com/jdx/hk/blob/main/AGENTS.md) when using coding agents.
