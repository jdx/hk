---
description: Run hk's checks for the harbour-master (CI), pick only the cargo that changed, call up extra watches, and haul in diagnostics worth reading.
sourceHash: 5d8ab9cc0a1c
---

# Continuous integration: the harbour-master

Use `hk check --all` to run the ship's checks against a checkout. The harbour-master (CI) must bring aboard hk and every tool the configured steps call on, just as a sailor's own machine does.

## One crew at the helm and for the harbour-master {#share-local-and-ci-checks}

As the song has it, the hook, the helm and the harbour-master all muster the self-same crew. Define a `check` hook that reuses yer linter mapping, the same lookouts ye post at the helm:

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

No `hk install` step is needed to call hk directly in CI. Keep check commands read-only, so a failing check, a squall, reports what needs to change. Mend the canvas with `hk fix` locally, review the changes, and commit them.

## Provision the ship {#set-up-the-environment}

With a committed `mise.toml`, these are the essential commands for the quartermaster in CI:

```sh
mise install
mise exec -- hk check --all
```

If yer linters are project dependencies, also run the package manager's install command and expose its executable directory to hk. The [mise integration guide](/mise_integration) shows a Node.js example.

Pin tool versions in the project so local and CI runs sail by the same rules. Keep hk's Pkl package imports versioned as well.

## Inspect only a branch's changes {#check-a-branch-s-changes}

A full check is the simplest baseline. For large ships, pick the files that differ between two references:

```sh
hk check --from-ref origin/main --to-ref HEAD
```

Replace `origin/main` with yer target branch, and make sure the checkout holds both references and enough history to compare them. Shallow clones may need an extra fetch.

Locally, `hk check --pr` picks the changes against the detected default branch. For the harbour-master, explicit references make the comparison easier to inspect.

::: tip Changed cargo is only a filter
Picking by reference chooses file paths; the commands still run against the cargo in the current checkout. It does not check out historical versions. A changed-file check also cannot work out every downstream effect of a change to shared configuration or a dependency.
:::

## Call up extra watches {#enable-additional-checks}

Use profiles, the watches, for checks too costly to run on every commit:

```pkl
["typecheck"] = (Builtins.tsc) {
  profiles = List("slow")
}
```

Call them up explicitly:

```sh
hk check --all --slow
hk check --all --profile ci --profile slow
```

A step with more than one positive profile needs all of them. A profile named `ci` is a label ye enable yerself; do not rely on its name to call up that watch automatically.

## Haul in useful diagnostics {#collect-useful-diagnostics}

```sh
hk check --all --no-fail-fast
hk check --all --plan --json
HK_TIMING_JSON=hk-timing.json hk check --all
```

`--no-fail-fast` collects the failures from the remaining steps too. A plan, the passage plan, shows the selected steps without executing them. The timing file records total and per-step wall time; when all hands haul at once their times overlap, so don't add parallel step durations together as a total.

Use `hk check --all --format jsonl` for structured execution events, `--sarif hk.sarif` for normalized diagnostics (only hands that set `diagnostic_format` produce them; see [diagnostics](/configuration#diagnostics)), or `--junit-xml hk.junit.xml` to report each step as a JUnit test case for CI test-result viewers. See [clockwork hands (coding agents)](/agents) for command effects and exact file lists.

See [troubleshooting](/logging) for the ship's log levels and traces, and for inspecting the configuration.
