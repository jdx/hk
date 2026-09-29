---
title: "hk completion"
description: "Write out a completion script, or install it where yer shell looks for it. hk prints any remaining shell setup instructions."
sourceHash: b53056afcc08
---

<!-- Pirate variant of docs/cli/completion.md; see docs/pirate/STYLE.md. -->

# `hk completion`

- **How to hail it:** `hk completion [--install] [--force] <SHELL>`
- **Effect:** read-only (looks, never touches)

Generate shell completion scripts, so yer shell can finish the bosun's calls for ye.

## Cargo it takes {#arguments}

- **`<SHELL>`** — The shell to generate completion for

## Flags to fly {#flags}

- **`--install`** — Install the script where this shell looks for it, instead of printing it

  Writes the script file and nothing else: no shell rc file and no PowerShell profile is edited. Where a shell needs a one-time line of its own — zsh's `fpath+=`, PowerShell's dot-source — it is printed for ye to add by hand.

  **Effect:** modifies state (lays hands on the cargo)

- **`--force`** — Replace a file at the target path that hk did not write

  **Effect:** modifies state (lays hands on the cargo)

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Generate a completion script, or install it in the place yer shell looks. hk prints any shell setup instructions that remain for ye.

```sh
hk completion bash
hk completion zsh --install
hk completion fish --install
```

## Further charts {#learn-more}

[Getting started: set sail](/getting_started) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
