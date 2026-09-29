---
title: "hk uninstall"
description: "Take down the bosun's pipes in the matching scope; this does not delete hk.pkl, the ship's charts."
sourceHash: 3fdc01e6d0e8
---

<!-- Pirate variant of docs/cli/uninstall.md; see docs/pirate/STYLE.md. -->

# `hk uninstall`

- **How to hail it:** `hk uninstall [--global]`
- **Effect:** destructive — may delete or irreversibly overwrite (what's cut away can't be hauled back)

Remove hk hooks: take down the bosun's pipes.

Removes hk's hooks from the current git repository, taking hk's pipes down from this ship, and clearing out both the legacy `.git/hooks/` shims and the config-based `hook.*` entries. Pass `--global` to remove them from the user's `~/.gitconfig` instead.

## Flags to fly {#flags}

- **`--global`** — Remove hk hooks from the user's global git config (`~/.gitconfig`).
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Remove the installation in the matching scope, one ship or the global config; this does not delete hk.pkl.

```sh
hk uninstall
hk uninstall --global
```

## Further charts {#learn-more}

[Getting started: set sail](/getting_started) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
