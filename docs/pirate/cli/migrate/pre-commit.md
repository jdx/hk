---
title: "hk migrate pre-commit"
description: "Write to a separate file while ye weigh up a migration. Steps that still sail under prek or pre-commit carry a comment saying why."
sourceHash: a85878c9d34b
---

<!-- Pirate variant of docs/cli/migrate/pre-commit.md; see docs/pirate/STYLE.md. -->

# `hk migrate pre-commit`

- **How to hail it:** `hk migrate pre-commit [FLAGS]`
- **Effect:** modifies state (lays hands on the cargo)

Change ships: migrate from pre-commit (or prek) to hk.

It reads a .pre-commit-config.yaml and writes an hk.pkl (the ship's charts) that runs the same hooks at the same git stages:

- Hooks from well-known repositories join the standing crew as hk builtins. For example,
  `ruff` from astral-sh/ruff-pre-commit becomes `Builtins.ruff`.
- Local `system`, `script`, and `fail` hooks sign on as native hk steps, with
  the same command and file filters.
- Every other hook keeps sailing under prek or pre-commit, which reads
  the original config. That includes hooks whose `args`,
  `additional_dependencies`, or filters a builtin cannot reproduce.
  Each of these steps carries a comment saying why it was not converted.

`manual` hooks run only with `hk check` and `hk fix`; no other call musters them.

## Flags to fly {#flags}

- **`-c --config <CONFIG>`** — Path to the .pre-commit-config.yaml to read

  **Unless ye say otherwise:** `.pre-commit-config.yaml`

- **`-f --force`** — Overwrite an existing hk.pkl file; the old charts are written over
- **`-o --output <OUTPUT>`** — Output path for the hk.pkl, where the new charts are written

  **Unless ye say otherwise:** `hk.pkl`

- **`--runner <RUNNER>`** — The tool that runs the hooks hk cannot convert.
  It defaults to pre-commit when only pre-commit is on PATH, and to prek otherwise.

  **Choose from:** `prek`, `pre-commit`

- **`--hk-pkl-root <HK_PKL_ROOT>`** — Root path for hk's pkl files (e.g. "pkl" for a local checkout, or a package URL prefix). If it's set, the generated config uses {root}/Config.pkl and {root}/Builtins.pkl
- **`-h --help`** — Print help, matey

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Write to a separate file while ye weigh up a migration. Steps that still sail under prek or pre-commit are marked with a comment saying why.

```sh
hk migrate pre-commit --output hk.migrated.pkl
HK_FILE=./hk.migrated.pkl hk check --all --plan
```

## Further charts {#learn-more}

[Set sail with getting started](/getting_started) · [Troubleshooting and the ship's log](/logging) · [All the bosun's calls](/cli/)
