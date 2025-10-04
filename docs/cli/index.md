# `hk`

**Usage**: `hk [FLAGS] <SUBCOMMAND>`

**Version**: 1.17.0

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

### `--trace`

Enable tracing spans and performance diagnostics

### `--json`

Output in JSON format

## Subcommands

- [`hk builtins`](/cli/builtins.md)
- [`hk cache clear`](/cli/cache/clear.md)
- [`hk check [FLAGS] [FILES]…`](/cli/check.md)
- [`hk completion <SHELL>`](/cli/completion.md)
- [`hk config <SUBCOMMAND>`](/cli/config.md)
- [`hk config dump [--format <FORMAT>]`](/cli/config/dump.md)
- [`hk config explain <KEY>`](/cli/config/explain.md)
- [`hk config get <KEY>`](/cli/config/get.md)
- [`hk config sources`](/cli/config/sources.md)
- [`hk fix [FLAGS] [FILES]…`](/cli/fix.md)
- [`hk init [-f --force] [--mise]`](/cli/init.md)
- [`hk install [--mise]`](/cli/install.md)
- [`hk run [FLAGS] [FILES]… <SUBCOMMAND>`](/cli/run.md)
- [`hk run commit-msg [FLAGS] <COMMIT_MSG_FILE> [FILES]…`](/cli/run/commit-msg.md)
- [`hk run pre-commit [FLAGS] [FILES]…`](/cli/run/pre-commit.md)
- [`hk run pre-push [FLAGS] [ARGS]…`](/cli/run/pre-push.md)
- [`hk run prepare-commit-msg [FLAGS] <ARGS>…`](/cli/run/prepare-commit-msg.md)
- [`hk test [FLAGS]`](/cli/test.md)
- [`hk uninstall`](/cli/uninstall.md)
- [`hk util <SUBCOMMAND>`](/cli/util.md)
- [`hk util check-byte-order-marker <FILES>…`](/cli/util/check-byte-order-marker.md)
- [`hk util check-case-conflict <FILES>…`](/cli/util/check-case-conflict.md)
- [`hk util check-executables-have-shebangs <FILES>…`](/cli/util/check-executables-have-shebangs.md)
- [`hk util check-merge-conflict <FILES>…`](/cli/util/check-merge-conflict.md)
- [`hk util check-symlinks <FILES>…`](/cli/util/check-symlinks.md)
- [`hk util fix-byte-order-marker <FILES>…`](/cli/util/fix-byte-order-marker.md)
- [`hk util mixed-line-ending [-f --fix] <FILES>…`](/cli/util/mixed-line-ending.md)
- [`hk util trailing-whitespace [-f --fix] <FILES>…`](/cli/util/trailing-whitespace.md)
- [`hk validate`](/cli/validate.md)
- [`hk version`](/cli/version.md)
