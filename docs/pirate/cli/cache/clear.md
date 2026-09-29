---
title: "hk cache clear"
description: "Sweep out hk's cache, its locker, when ye're hunting down stale evaluated configuration."
sourceHash: 0663c9b7f3bd
---

<!-- Pirate variant of docs/cli/cache/clear.md; see docs/pirate/STYLE.md. -->

# `hk cache clear`

- **How to hail it:** `hk cache clear`
- **Effect:** modifies state (it touches, as well as looks)

Clear out the cache directory, hk's own locker rather than yer project's files, and sweep it clean as a scrubbed deck

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Clear hk's cache when ye're hunting down stale evaluated configuration.

```sh
hk cache clear
hk validate
```

## Further charts {#learn-more}

[Getting started: get under way](/getting_started) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
