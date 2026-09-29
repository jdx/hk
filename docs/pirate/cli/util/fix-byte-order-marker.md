---
title: "hk util fix-byte-order-marker"
description: "This call lays hands on the files ye give it, cutting away a UTF-8 BOM."
sourceHash: fb6475516a01
---

<!-- Pirate variant of docs/cli/util/fix-byte-order-marker.md; see docs/pirate/STYLE.md. -->

# `hk util fix-byte-order-marker`

- **How to hail it:** `hk util fix-byte-order-marker <FILES>…`
- **Effect:** modifies state (lays hands on the cargo)

Removes the UTF-8 byte order marker (BOM), that stowaway at the very start of a file.

## Cargo it takes {#arguments}

- **`<FILES>…`** — The files to cut the BOM from

## Flags to fly {#flags}

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Mind ye: this call changes the files ye hand it, removing a UTF-8 BOM.

```sh
hk util fix-byte-order-marker README.md
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
