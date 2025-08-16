# xx

[![Crates.io](https://img.shields.io/crates/v/xx.svg)](https://crates.io/crates/xx)
[![Documentation](https://docs.rs/xx/badge.svg)](https://docs.rs/xx)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A collection of useful Rust macros and small utility functions to make common tasks easier.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
xx = "2.1"
```

## Features

The library provides several optional features that can be enabled as needed:

- **`archive`** - Archive extraction support (tar.gz, tar.bz2, tar.xz, zip, gzip)
- **`glob`** - File globbing support
- **`hash`** - SHA256 hashing utilities
- **`http`** - HTTP client functionality
- **`fslock`** - File system locking

Enable features in your `Cargo.toml`:

```toml
[dependencies]
xx = { version = "2.1", features = ["archive", "glob", "hash"] }
```

## Modules

### Core Modules (Always Available)

- **`file`** - Enhanced file operations with better error handling
- **`process`** - Process execution utilities
- **`git`** - Git repository operations
- **`env`** - Environment variable parsing utilities
- **`context`** - Context management utilities
- **`error`** - Error types and result helpers

### Optional Modules

- **`archive`** - Archive extraction (requires `archive` feature)
- **`hash`** - SHA256 hashing (requires `hash` feature)
- **`http`** - HTTP client (requires `http` feature)
- **`fslock`** - File locking (requires `fslock` feature)

## Examples

### File Operations

```rust
use xx::file;

// Read and write files
let content = file::read_to_string("config.toml")?;
file::write("output.txt", "Hello, world!")?;

// Append to files
file::append("log.txt", "New log entry\n")?;

// Directory operations
file::mkdirp("path/to/directory")?;
file::copy_dir_all("src_dir", "dest_dir")?;
assert!(file::is_empty_dir("some_dir")?);

// Find executables
if let Some(git) = file::which("git") {
    println!("Git found at: {}", git.display());
}
```

### Environment Variables

```rust
use xx::env;

// Parse boolean environment variables
if env::var_is_true("VERBOSE") {
    println!("Verbose mode enabled");
}

// Parse paths with tilde expansion
if let Some(config_dir) = env::var_path("CONFIG_DIR") {
    // ~/.config expands to /home/user/.config
}

// Parse numeric values
let threads = env::var_u32("NUM_THREADS").unwrap_or(4);
let timeout = env::var_i64("TIMEOUT_MS").unwrap_or(5000);
```

### Process Execution

```rust
use xx::process;

// Run shell commands
let output = process::sh("ls -la")?;

// Build and run commands
let result = process::cmd("git", &["status"])
    .read()?;
```

### Git Operations

```rust
use xx::git::{Git, CloneOptions};

// Clone a repository
let opts = CloneOptions::default().branch("main");
let repo = xx::git::clone("https://github.com/user/repo", "/tmp/repo", &opts)?;

// Work with existing repository
let git = Git::new("/path/to/repo".into());
let branch = git.current_branch()?;
let sha = git.current_sha()?;
```

## License

MIT - See [LICENSE](LICENSE) for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Links

- [Documentation](https://docs.rs/xx)
- [Crates.io](https://crates.io/crates/xx)
- [Repository](https://github.com/jdx/xx)