---
title: "hk config sources"
description: "See which configuration source outranks which, then look over the resolved values."
sourceHash: 6411a510b08b
---

<!-- Pirate variant of docs/cli/config/sources.md; see docs/pirate/STYLE.md. -->

# `hk config sources`

- **How to hail it:** `hk config sources`
- **Effect:** read-only (looks, never touches)

Show the precedence order of the configuration sources, the ship's chain of command

Lists every configuration source from highest to lowest precedence: the one with the final say first, the one that yields to all the rest last.

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Show the precedence first, the chain of command, then look over the resolved values.

```sh
hk config sources
hk config dump
```

## Further charts {#learn-more}

[Configuration guide: the ship's charts](/configuration) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
