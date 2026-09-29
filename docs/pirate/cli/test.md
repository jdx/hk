---
title: "hk test"
description: "Put the hands through their drills: list step-defined tests, or run only the tests belonging to one configured step."
sourceHash: 6ad995c79824
---

<!-- Pirate variant of docs/cli/test.md; see docs/pirate/STYLE.md. -->

# `hk test`

- **How to hail it:** `hk test [FLAGS]`

Run step-defined tests: drills written into the steps themselves.

## Flags to fly {#flags}

- **`--list`** — List the tests without running them
- **`--name <NAME>…`** — Filter by test name (repeatable)
- **`--step <STEP>…`** — Filter by step name, to drill only those hands (repeatable)
- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

List the step-defined tests, or drill only the tests belonging to one configured step.

```sh
hk test --list
hk test --step whitespace
hk test --name 'accepts clean text'
```

## Further charts {#learn-more}

[Getting started: set sail](/getting_started) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
