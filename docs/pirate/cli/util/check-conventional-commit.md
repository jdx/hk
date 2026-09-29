---
title: "hk util check-conventional-commit"
description: "Read the name she's christened with from a file and see it's proper: check a file holding a commit message."
sourceHash: b713c0c053c0
---

<!-- Pirate variant of docs/cli/util/check-conventional-commit.md; see docs/pirate/STYLE.md. -->

# `hk util check-conventional-commit`

- **How to hail it:** `hk util check-conventional-commit [--allowed-types <ALLOWED_TYPES>] <COMMIT_MSG_FILE>`
- **Effect:** read-only (looks, never touches)

Checks that the name ye christen her with (the commit message) follows the Conventional Commits format.

Titles starting with `fixup! `, `squash! `, or `amend! ` (temporary commits made for `git rebase --autosquash`) skip validation and are waved through.

## Cargo it takes {#arguments}

- **`<COMMIT_MSG_FILE>`** — The commit message file to check

## Flags to fly {#flags}

- **`--allowed-types <ALLOWED_TYPES>`** — Comma-separated list of commit types to let aboard

  **Unless ye say otherwise:** `build,chore,ci,docs,feat,fix,perf,refactor,revert,style,test`

- **`-h --help`** — Print help, for when ye've lost yer bearings

<!-- hk documentation examples -->

## Tales from the deck {#examples}

Check a file holding a commit message, such as "feat: hoist the sails".

```sh
hk util check-conventional-commit .git/COMMIT_EDITMSG
```

## Further charts {#learn-more}

[Built-in linters and utilities: the standing crew](/builtins) · [Troubleshooting: the ship's log](/logging) · [All the bosun's calls](/cli/)
