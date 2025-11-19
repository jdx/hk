# `hk run`

- **Usage**: `hk run [FLAGS] [FILES]… <SUBCOMMAND>`
- **Aliases**: `r`

Run a hook

## Arguments

### `[FILES]…`

Run on specific files

## Flags

### `-a --all`

Run on all files instead of just staged files

### `-c --check`

Run check command instead of fix command

### `-e --exclude… <EXCLUDE>`

Exclude files that otherwise would have been selected

### `-f --fix`

Run fix command instead of run command This is the default behavior unless HK_FIX=0

### `-g --glob… <GLOB>`

Run on files that match these glob patterns

### `-P --plan`

Print the plan instead of running the hook

### `-S --step… <STEP>`

Run only specific step(s)

### `--fail-fast`

Abort on first failure

### `--from-ref <FROM_REF>`

Start reference for checking files (requires --to-ref)

### `--no-fail-fast`

Continue on failures (opposite of --fail-fast)

### `--skip-step… <STEP>`

Skip specific step(s)

### `--stash <STASH>`

Stash method to use for git hooks

**Choices:**

- `git`
- `patch-file`
- `none`

### `--to-ref <TO_REF>`

End reference for checking files (requires --from-ref)

## Subcommands

- [`hk run commit-msg [FLAGS] <COMMIT_MSG_FILE> [FILES]…`](/cli/run/commit-msg.md)
- [`hk run pre-commit [FLAGS] [FILES]…`](/cli/run/pre-commit.md)
- [`hk run pre-push [FLAGS] [ARGS]…`](/cli/run/pre-push.md)
- [`hk run prepare-commit-msg [FLAGS] <ARGS>…`](/cli/run/prepare-commit-msg.md)
