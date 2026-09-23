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

mise wraps `cargo` with [mbx](https://mr-boxington.jdx.dev), so compiled work is
shared across checkouts. `mise run` tasks and `mise exec -- cargo …` use the
wrapper; plain `cargo` does too once mise is [activated in your
shell](https://mise.jdx.dev/getting-started.html#activate-mise). Builds that set
`MBX_DISABLE=1` skip the cache, except `mise run perf:build`, which calls `mbx`
directly to reuse the perf cache.
