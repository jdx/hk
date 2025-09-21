# `hk run prepare-commit-msg`

- **Usage**: `hk run prepare-commit-msg [FLAGS] <ARGS>…`
- **Aliases**: `pcm`

## Arguments

### `<COMMIT_MSG_FILE>`

The path to the file that contains the commit message so far

### `[SOURCE]`

The source of the commit message (e.g., "message", "template", "merge")

### `[SHA]`

The SHA of the commit being amended (if applicable)

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

### `--from-ref <FROM_REF>`

Start reference for checking files (requires --to-ref)

### `--to-ref <TO_REF>`

End reference for checking files (requires --from-ref)

### `-g --glob… <GLOB>`

Run on files that match these glob patterns

### `-P --plan`

Print the plan instead of running the hook

### `-S --step… <STEP>`

Run only specific step(s)

### `--skip-step… <STEP>`

Skip specific step(s)

### `--fail-fast`

Abort on first failure

### `--no-fail-fast`

Continue on failures (opposite of --fail-fast)

### `--stash <STASH>`

Stash method to use for git hooks

**Choices:**

- `git`
- `patch-file`
- `none`
