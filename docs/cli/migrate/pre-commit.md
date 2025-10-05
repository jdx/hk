# `hk migrate pre-commit`

- **Usage**: `hk migrate pre-commit [FLAGS]`

Migrate from pre-commit to hk

## Flags

### `-c --config <CONFIG>`

Path to .pre-commit-config.yaml

### `-o --output <OUTPUT>`

Output path for hk.pkl

### `-f --force`

Overwrite existing hk.pkl file

### `--hk-pkl-root <HK_PKL_ROOT>`

Root path for hk pkl files (e.g., "pkl" for local, or package URL prefix) If specified, will use {root}/Config.pkl and {root}/Builtins.pkl
