---
title: "hk install"
description: "Choose where to fit the bosun's pipe: on this ship alone, or globally, on every ship on the machine, with Git 2.54+. Add --mise when Git needs mise, the quartermaster, to provide the tools."
sourceHash: 7eb78968b9a7
---

<!-- Pirate variant of docs/cli/install.md; see docs/pirate/STYLE.md. -->

# `hk install`

- **How to hail it:** `hk install [FLAGS]`
- **Also answers to:** `i`
- **Effect:** modifies state (lays hands on the cargo)

Set up git hooks to run hk: rig the bosun's pipes that call all hands on deck.

The recommended setup is `hk install --global`. It installs the hooks once into the user's `~/.gitconfig`, so every ship on the machine picks them up automatically. In a project without an `hk.pkl`, with no charts aboard, the installed hook exits silently and does nothing (a no-op), so 'tis safe to enable everywhere ye sail. Requires Git 2.54+.

Without `--global`, the hooks are installed into the current ship only. On Git 2.54+ this uses config-based hooks (`hook.<name>.command`), which leave `.git/hooks/` untouched and sail cleanly alongside other hook managers. On older Git it falls back to writing script shims.

If hk is already configured globally (any `hook.hk-*` entry in `~/.gitconfig`), the per-repo install is skipped, and any stale local hooks are cleaned up. That way the global install stays the single source of truth, and hk doesn't pipe all hands twice for one event. Pass `--force-local` to install local hooks anyway.

## Flags to fly {#flags}

- **`--force-local`** — Install local hooks even when hk is already configured globally
  (any `hook.hk-*` entry in `~/.gitconfig`). By default a per-repo
  install is skipped in that case, so hk doesn't fire twice per
  event. Not compatible with `--global`.
- **`--global`** — Recommended. Install at user level (~/.gitconfig) so every ship
  on this machine gets hk hooks. Requires Git 2.54 or newer. In
  repos without an `hk.pkl`, the installed hook is a silent no-op: the pipe sounds and nobody is called.
- **`--legacy`** — Force the old rigging: legacy `.git/hooks/` script shims instead of Git
  2.54+ config-based hooks. Not compatible with `--global`.
- **`--mise`** — Run hooks through `mise x`, so the tools the quartermaster (mise) manages are aboard
  without activating mise in the shell.

  Set HK_MISE=1 as a standing order to make this the default behavior.

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Choose a local installation, for one ship, or a global one on Git 2.54+, for the whole fleet. Add --mise when Git needs mise to provide the tools.

```sh
hk install

# Alternative: install once for all repositories
hk install --global
```

## Further charts {#learn-more}

[Getting started: set sail](/getting_started) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
