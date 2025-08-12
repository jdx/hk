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

### `-f --fix`

Run fix command instead of run command This is the default behavior unless HK_FIX=0

### `-c --check`

Run run command instead of fix command

### `-e --exclude… <EXCLUDE>`

Exclude files that otherwise would have been selected

### `--exclude-glob… <EXCLUDE_GLOB>`

Exclude files that match these glob patterns that otherwise would have been selected

### `--from-ref <FROM_REF>`

Start reference for checking files (requires --to-ref)

### `--to-ref <TO_REF>`

End reference for checking files (requires --from-ref)

### `-g --glob… <GLOB>`

Run on files that match these glob patterns

### `-P --plan`

Print the plan instead of running the hook

### `-S --step… <STEP>`

Run specific step(s)

## Subcommands

- [`hk run commit-msg [FLAGS] <COMMIT_MSG_FILE> [FILES]…`](/cli/run/commit-msg.md)
- [`hk run pre-commit [FLAGS] [FILES]…`](/cli/run/pre-commit.md)
- [`hk run pre-push [FLAGS] [ARGS]…`](/cli/run/pre-push.md)
- [`hk run prepare-commit-msg [FLAGS] <ARGS>…`](/cli/run/prepare-commit-msg.md)
