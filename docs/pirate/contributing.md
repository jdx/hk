---
description: Sign aboard hk. Fit out for development, run focused checks, edit the generated documentation, and ready yer contribution for review.
sourceHash: 752a31b1098f
---

# Signing aboard to contribute

_So all you hands who would sign aboard:_ bug fixes, documentation improvements, and builtin definitions for the standing crew are all welcome. For a substantial feature or a change in behavior, talk over the course ye mean to set before ye invest in building it.

## What to expect at review {#review-expectations}

Open a [discussion](https://github.com/jdx/hk/discussions) or hail yer shipmates in [Discord](https://discord.gg/UBa7pJUN7Z) before starting a change whose scope or design isn't obvious. hk has a deliberate scope and holds its course; the maintainer may decline features that add complexity or long-term maintenance without a clear fit.

Before ye ask for review, make sure CI passes, so the harbour-master waves ye through, and address the automated review comments. In the PR, explain the problem, the resulting behavior, and how ye validated it. A contribution should be complete enough to assess without extensive coaching.

The maintainer handles a high volume of contributions across many ships (projects). Feedback or a rejection may be brief, and an uncertain fit can be reason enough to decline a change.

## Fitting out for development {#development-setup}

Install [mise](https://mise.jdx.dev/) (the quartermaster) and a Rust toolchain compatible with the ship's `Cargo.toml`, then:

```sh
git clone https://github.com/jdx/hk.git
cd hk
mise install
mise run build
```

The build task generates the builtin registry, the crew roster, before compiling hk. Development tasks put the local debug binary on `PATH`, ready to hand.

## Run focused inspections {#run-focused-checks}

| The task                          | The call                             |
| --------------------------------- | ------------------------------------ |
| Build                             | `mise run build`                     |
| Rust tests                        | `mise run test:cargo`                |
| One Rust test                     | `cargo test test_name`               |
| Bats integration tests            | `mise run test:bats`                 |
| One Bats file                     | `mise run test:bats test/check.bats` |
| The full test suite               | `mise run test`                      |
| Lint: send up the lookouts        | `hk check --all`                     |
| Lint, Clippy included             | `hk check --all --slow`              |
| Mend the formatting (apply fixes) | `hk fix --all`                       |

Run the checks that suit yer change. Integration tests sail in isolated temporary repositories, little ships of their own, and put the Git backends through their paces. See the [test-suite guide](https://github.com/jdx/hk/blob/main/test/README.md) for fixtures and cache behavior.

## Add a builtin to the standing crew {#add-a-builtin}

1. Add `pkl/builtins/<name>.pkl` with the new hand's metadata, file patterns, and commands.
2. Define Pkl-level tests in the step's `tests` field. Use `TestMaker` from `pkl/builtins/test/helpers.pkl` for the standard check/fix patterns.
3. Add a `mise tool-stub` script in `test/builtin_tool_stubs/` if the tool isn't already aboard.
4. Regenerate and build the ship with `mise run build`.
5. Run `mise run test:bats test/builtins_tests.bats`, or use `hk test --step <name>` with a configuration that brings the builtin aboard.

Tests should prove meaningful behavior: a clean check, a failing check, and the expected result of a fix when the hand supports one. A builtin with `check_diff` also needs a diff test (`TestMaker.diffPass` or `diffFail`), which applies the patch `check_diff` prints and fails if `git apply` rejects it. Fix tests can't catch that, because they run `fix` directly, and `test/builtins_tests.bats` fails for a tested builtin without a diff test. Avoid enabling batching or slipping the lashings (bypassing locks) until ye've confirmed how the tool behaves.

## Keep the documentation shipshape {#edit-documentation}

The website sails on VitePress. Run these from the repository root:

```sh
mise run docs        # Generate reference content and start the dev server
mise run docs:build  # Generate and build the production site
```

Each kind of page has one true source. Edit it there:

| The page                                | Edit it here                                                                  |
| --------------------------------------- | ----------------------------------------------------------------------------- |
| README and guides                       | `README.md`, `docs/*.md`                                                      |
| Landing page and theme                  | `docs/.vitepress/theme/`                                                      |
| Navigation                              | `docs/.vitepress/config.mts`                                                  |
| Schema reference                        | Documentation comments in `pkl/Config.pkl`                                    |
| Settings reference                      | `docs` strings in `settings.toml`                                             |
| Builtin catalogue, the crew roster      | Metadata and definitions in `pkl/builtins/`                                   |
| CLI reference, the bosun's calls        | Rust CLI help and usage definitions; examples in `scripts/enrich-cli-docs.py` |
| Downloadable examples, ships in bottles | `docs/public/*.pkl`, included directly by their guide pages                   |

`mise run docs:gen` generates `docs/gen/`. To regenerate the CLI reference, use `mise run render:usage`; mind that task also stages its generated outputs, loading them aboard for yer next voyage. Run `scripts/generate-examples.sh` to validate the downloadable examples and the documentation includes that pull them in.

Keep an example complete when it's meant to be copied. Label fragments, list the outside tools an example needs, and use one package version for both schema and builtin imports. Verify internal links so none runs aground, and inspect affected layouts at narrow and wide widths.

## Christening commits and pull requests {#commit-messages-and-pull-requests}

Christen each commit in the Conventional Commits style, with a lowercase, imperative description:

- `fix(step): handle missing files`
- `feat(builtins): add a formatter`
- `docs: clarify hook installation`
- `chore: update CI tooling`

Use `fix` for changes to CLI behavior; use `chore` for CI (the harbour-master) and infrastructure. Add a command or subsystem scope when it applies.

Open a PR ready for review, with focused changes and a concise summary of how ye validated them. When clockwork hands (coding agents) help ye, follow the repository's [agent guidelines](https://github.com/jdx/hk/blob/main/AGENTS.md).
