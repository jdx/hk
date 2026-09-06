---
description: Run hk checks in CI, select changed files, enable profiles, and keep diagnostics useful.
---

# Continuous integration

Use `hk check --all` to run the project’s checks against a checkout. CI must install hk and every tool used by the configured steps, just as a developer’s machine does.

## Share local and CI checks

Define a `check` hook that reuses your linter mapping:

```pkl
hooks {
  ["check"] { steps = linters }
}
```

Then run:

```sh
hk validate
hk check --all
```

No `hk install` step is needed to invoke hk directly in CI. Keep check commands read-only so failures report what needs to change. Use `hk fix` locally, review the changes, and commit them.

## Set up the environment

With a committed `mise.toml`, the essential CI commands are:

```sh
mise install
mise exec -- hk check --all
```

If your linters are project dependencies, also run the package manager’s install command and expose its executable directory to hk. The [mise integration guide](/mise_integration) shows a Node.js example.

Pin tool versions in the project so local and CI runs use the same rules. Keep hk’s Pkl package imports versioned as well.

## Check a branch’s changes

A full check is the simplest baseline. For large repositories, select files that differ between two references:

```sh
hk check --from-ref origin/main --to-ref HEAD
```

Replace `origin/main` with your target branch and ensure the checkout contains both references and enough history to compare them. Shallow clones may need an additional fetch.

Locally, `hk check --pr` selects changes against the detected default branch. For CI, explicit references make the comparison easier to inspect.

::: tip Changed files are a filter
Reference selection chooses file paths; the commands run against the current checkout. It does not check out historical versions. A changed-file check also cannot determine every downstream effect of a shared configuration or dependency change.
:::

## Enable additional checks

Use profiles for checks that are too expensive for every commit:

```pkl
["typecheck"] = (Builtins.tsc) {
  profiles = List("slow")
}
```

Enable them explicitly:

```sh
hk check --all --slow
hk check --all --profile ci --profile slow
```

A step with multiple positive profiles requires all of them. A profile named `ci` is a label you enable; do not rely on its name to activate it automatically.

## Collect useful diagnostics

```sh
hk check --all --no-fail-fast
hk check --all --plan --json
HK_TIMING_JSON=hk-timing.json hk check --all
```

`--no-fail-fast` collects failures from remaining steps. A plan shows selected steps without executing them. The timing file records total and per-step wall time; parallel step durations should not be added together as a total.

Use `hk check --all --format jsonl` for structured execution events or `--sarif hk.sarif` for normalized diagnostics. See [coding agents](/agents) for command effects and exact file lists.

See [troubleshooting](/logging) for log levels, traces, and configuration inspection.
