# `hk util`

- **Usage**: `hk util <SUBCOMMAND>`

Utility commands for file operations

## Subcommands

### `trailing-whitespace`

Detect and remove trailing whitespace from files

**Usage**: `hk util trailing-whitespace [FLAGS] <FILES>…`

#### Arguments

**`<FILES>…`**

Files to check or fix

#### Flags

**`-f --fix`**

Remove trailing whitespace instead of just detecting it

#### Examples

```bash
# Check for trailing whitespace
hk util trailing-whitespace file1.txt file2.txt

# Fix trailing whitespace
hk util trailing-whitespace --fix *.txt

# Use in hk.pkl via builtin
hooks {
  ["pre-commit"] {
    steps {
      ["trailing-ws"] = Builtins.trailing_whitespace
    }
  }
}
```

#### Features

- Cross-platform (works on Windows, macOS, Linux)
- Automatically skips non-text files
- Detects spaces, tabs, and mixed trailing whitespace
- Exit codes:
  - Check mode: Exit 1 if issues found, 0 if clean
  - Fix mode: Exit 0 on success

#### Implementation

Uses pure Rust implementation instead of shell scripts for:
- Better cross-platform compatibility
- Improved testability with unit tests
- Consistent behavior across platforms
- No external dependencies
