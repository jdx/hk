# `hk`

**Usage**: `hk [FLAGS] <SUBCOMMAND>`

**Version**: 1.2.0

- **Usage**: `hk [FLAGS] <SUBCOMMAND>`

## Global Flags

### `--hkrc <PATH>`

Path to user configuration file

### `-j --jobs <JOBS>`

Number of jobs to run in parallel

### `-p --profile… <PROFILE>`

Profiles to enable/disable prefix with ! to disable e.g. --profile slow --profile !fast

### `-s --slow`

Shorthand for --profile=slow

### `-v --verbose…`

Enables verbose output

### `-n --no-progress`

Disables progress output

### `-q --quiet`

Suppresses output

### `--silent`

Suppresses all output

## Subcommands

- [`hk builtins`](/cli/builtins.md)
- [`hk cache clear`](/cli/cache/clear.md)
- [`hk check [FLAGS] [FILES]…`](/cli/check.md)
- [`hk completion <SHELL>`](/cli/completion.md)
- [`hk config`](/cli/config.md)
- [`hk fix [FLAGS] [FILES]…`](/cli/fix.md)
- [`hk init [-f --force] [--mise]`](/cli/init.md)
- [`hk install [--mise]`](/cli/install.md)
- [`hk run [FLAGS] [FILES]… <SUBCOMMAND>`](/cli/run.md)
- [`hk run commit-msg [FLAGS] <COMMIT_MSG_FILE> [FILES]…`](/cli/run/commit-msg.md)
- [`hk run pre-commit [FLAGS] [FILES]…`](/cli/run/pre-commit.md)
- [`hk run pre-push [FLAGS] [ARGS]…`](/cli/run/pre-push.md)
- [`hk run prepare-commit-msg [FLAGS] <ARGS>…`](/cli/run/prepare-commit-msg.md)
- [`hk uninstall`](/cli/uninstall.md)
- [`hk validate`](/cli/validate.md)
- [`hk version`](/cli/version.md)
