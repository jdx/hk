---
title: "hk init"
description: "Draw up the ship's charts (a configuration), then validate them. The tools ye pick must be brought aboard separately."
sourceHash: c3583a09efe0
---

<!-- Pirate variant of docs/cli/init.md; see docs/pirate/STYLE.md. -->

# `hk init`

- **How to hail it:** `hk init [FLAGS]`
- **Effect:** modifies state (it touches, as well as looks)

Generate a new hk.pkl file for a project: fresh charts for the ship.

## Flags to fly {#flags}

- **`-f --force`** — Overwrite an existing hk.pkl file, old charts and all
- **`-i --interactive`** — Interactive mode: pick yer lookouts (linters) and hooks by hand
- **`--mise`** — Generate a mise.toml file with hk configured, for the quartermaster (mise)

  Set HK_MISE=1 as a standing order to make this the default behavior.

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Draw up a configuration, then validate it. Install the tools ye selected separately; they don't come aboard on their own.

```sh
hk init
hk validate

# Choose linters and hooks interactively
hk init --interactive
```

## Further charts {#learn-more}

[Getting started: get under way](/getting_started) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
