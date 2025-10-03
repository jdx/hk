# `hk util`

- **Usage**: `hk util <SUBCOMMAND>`

Utility commands for file operations

## Subcommands

### `check-executables-have-shebangs`

Check that executable files have shebangs

**Usage**: `hk util check-executables-have-shebangs <FILES>…`

#### Arguments

**`<FILES>…`**

Files to check

#### Examples

```bash
# Check if executables have shebangs
hk util check-executables-have-shebangs script.sh

# Use in hk.pkl via builtin
hooks {
  ["pre-commit"] {
    steps {
      ["executable-shebangs"] = Builtins.check_executables_have_shebangs
    }
  }
}
```

#### Features

- Detects executable files without shebang (`#!`) lines
- Only checks files with execute permission set
- Automatically skips binary files
- Accepts any shebang format (e.g., `#!/bin/bash`, `#!/usr/bin/env python`)
- Exit code 1 if issues found, 0 if clean

### `check-symlinks`

Check for broken symlinks

**Usage**: `hk util check-symlinks <FILES>…`

#### Arguments

**`<FILES>…`**

Files to check

#### Examples

```bash
# Check for broken symlinks
hk util check-symlinks link1 link2

# Use in hk.pkl via builtin
hooks {
  ["pre-commit"] {
    steps {
      ["symlinks"] = Builtins.check_symlinks
    }
  }
}
```

#### Features

- Detects symlinks that point to non-existent targets
- Works with both file and directory symlinks
- Only flags broken symlinks, not regular files
- Exit code 1 if broken symlinks found, 0 if clean

### `mixed-line-ending`

Detect and fix mixed line endings

**Usage**: `hk util mixed-line-ending [FLAGS] <FILES>…`

#### Arguments

**`<FILES>…`**

Files to check or fix

#### Flags

**`-f --fix`**

Fix mixed line endings by normalizing to LF

#### Examples

```bash
# Check for mixed line endings
hk util mixed-line-ending file.txt

# Fix mixed line endings
hk util mixed-line-ending --fix *.txt

# Use in hk.pkl via builtin
hooks {
  ["pre-commit"] {
    steps {
      ["mixed-endings"] = Builtins.mixed_line_ending
    }
  }
}
```

#### Features

- Detects files with both CRLF and LF line endings
- Normalizes to LF when fixing
- Automatically skips binary files
- Exit codes:
  - Check mode: Exit 1 if mixed endings found, 0 if clean
  - Fix mode: Exit 0 on success

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
