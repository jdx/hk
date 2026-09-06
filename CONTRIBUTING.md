# Contributing to hk

Read the [contributing guide](docs/contributing.md) for review expectations, development setup, and how to add a builtin. It is also available on the [documentation website](https://hk.jdx.dev/contributing).

To get a checkout ready:

```sh
mise install
mise run build
mise run test
```

For a documentation change, use `mise run docs` to preview the site and `mise run docs:build` to validate it. Generated reference content has its own source files; see [editing documentation](docs/contributing.md#edit-documentation).

PR titles must use Conventional Commits; use the same format for intermediate commits where practical, for example `fix(step): handle missing files` or `docs: clarify hook installation`.

## mbx build cache

`mise install` installs [mbx](https://mr-boxington.jdx.dev). The normal
`mise run build`, `mise run test:cargo`, and `mise run lint` workflows activate
its transparent Cargo wrapper and therefore use the cache while invoking Cargo
normally. Standalone Cargo commands require an activated mise shell. To bypass
mbx without skipping or weakening a check, prefix the
equivalent Cargo command with `MBX_DISABLE=1`:

```sh
MBX_DISABLE=1 cargo build
MBX_DISABLE=1 cargo test --all --all-features
MBX_DISABLE=1 cargo clippy --manifest-path Cargo.toml --quiet -- -D warnings
CARGO_BUILD_WARNINGS=deny MBX_DISABLE=1 cargo check --quiet
```

If bypassed Cargo succeeds where the wrapper fails, or mbx introduces a papercut, please start a
[mr-boxington Discussion](https://github.com/jdx/mr-boxington/discussions).
Include the repository and commit, operating system, `mbx --version`,
`mbx doctor`, and both commands and their output. Before posting, redact
secrets, absolute cache paths, remote URLs, namespaces, and other sensitive or
identifying details.
