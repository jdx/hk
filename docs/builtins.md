---
outline: "deep"
---

# Built-in Linters Reference

hk provides 70+ pre-configured linters and formatters through the `Builtins` module. These are production-ready configurations that work out of the box.

## Usage

Import and use builtins in your `hk.pkl`:

```pkl
amends "package://github.com/jdx/hk/releases/download/v1.19.0/hk@1.19.0#/Config.pkl"
import "package://github.com/jdx/hk/releases/download/v1.19.0/hk@1.19.0#/Builtins.pkl"

hooks {
  ["pre-commit"] {
    steps {
      ["prettier"] = Builtins.prettier
      ["eslint"] = Builtins.eslint
    }
  }
}
```

You can also customize builtins:

```pkl
["prettier"] = (Builtins.prettier) {
  batch = false  // Override the default batch setting
  glob = List("*.js", "*.ts")  // Override file patterns
}
```

## Available Builtins

### JavaScript/TypeScript

#### `prettier`
- **Files:** `*.js`, `*.jsx`, `*.ts`, `*.tsx`, `*.css`, `*.scss`, `*.html`, `*.json`, `*.yaml`, `*.md`, and more
- **Features:** Batch processing, list-different optimization
- **Commands:**
  - Check: `prettier --check {{files}}`
  - Fix: `prettier --write {{files}}`

#### `eslint`
- **Files:** `*.js`, `*.jsx`, `*.ts`, `*.tsx`
- **Features:** Batch processing for performance
- **Commands:**
  - Check: `eslint {{files}}`
  - Fix: `eslint --fix {{files}}`

#### `tsc`
- **Files:** TypeScript projects
- **Features:** Type checking only (no emit)
- **Command:** `tsc --noEmit`

#### `tsserver`
- **Files:** `*.ts`, `*.tsx`, `*.js`, `*.jsx`
- **Features:** TypeScript language server diagnostics
- **Command:** `tsserver --format {{files}}`

#### `biome`
- **Files:** `*.js`, `*.jsx`, `*.ts`, `*.tsx`, `*.json`
- **Features:** Fast formatter and linter
- **Commands:**
  - Check: `biome check {{files}}`
  - Fix: `biome check --apply {{files}}`

#### `deno`
- **Files:** `*.ts`, `*.tsx`, `*.js`, `*.jsx`
- **Features:** Deno formatter and linter
- **Commands:**
  - Check: `deno fmt --check {{files}} && deno lint {{files}}`
  - Fix: `deno fmt {{files}} && deno lint --fix {{files}}`

#### `deno_check`
- **Files:** `*.ts`, `*.tsx`
- **Features:** Deno type checker
- **Command:** `deno check {{files}}`

#### `standard_js`
- **Files:** `*.js`, `*.jsx`
- **Features:** JavaScript Standard Style
- **Commands:**
  - Check: `standard {{files}}`
  - Fix: `standard --fix {{files}}`

#### `xo`
- **Files:** `*.js`, `*.jsx`, `*.ts`, `*.tsx`
- **Features:** JavaScript/TypeScript linter with great defaults
- **Commands:**
  - Check: `xo {{files}}`
  - Fix: `xo --fix {{files}}`

#### `ox_lint`
- **Files:** `*.js`, `*.jsx`, `*.ts`, `*.tsx`
- **Features:** Oxidation compiler linter
- **Commands:**
  - Check: `oxlint {{files}}`
  - Fix: `oxlint --fix {{files}}`

### Python

#### `black`
- **Files:** `*.py`, `*.pyi`
- **Features:** Opinionated Python formatter
- **Commands:**
  - Check: `black --check {{files}}`
  - Fix: `black {{files}}`

#### `ruff`
- **Files:** `*.py`, `*.pyi`
- **Features:** Fast Python linter
- **Commands:**
  - Check: `ruff check {{files}}`
  - Fix: `ruff check --fix {{files}}`

#### `ruff_format`
- **Files:** `*.py`
- **Features:** Fast Python formatter (part of ruff)
- **Commands:**
  - Check: `ruff format --check {{files}}`
  - Fix: `ruff format {{files}}`

#### `isort`
- **Files:** `*.py`
- **Features:** Python import sorter
- **Commands:**
  - Check: `isort --check-only {{files}}`
  - Fix: `isort {{files}}`

#### `flake8`
- **Files:** `*.py`
- **Features:** Python style guide enforcement
- **Command:** `flake8 {{files}}`

#### `pylint`
- **Files:** `*.py`
- **Features:** Python code analysis
- **Command:** `pylint {{files}}`

#### `mypy`
- **Files:** `*.py`
- **Features:** Static type checker for Python
- **Command:** `mypy {{files}}`

### Rust

#### `cargo_fmt`
- **Files:** `*.rs`
- **Features:** Rust code formatter
- **Commands:**
  - Check: `cargo fmt -- --check`
  - Fix: `cargo fmt`

#### `rustfmt`
- **Files:** `*.rs`
- **Features:** Rust code formatter (alias for cargo_fmt)
- **Commands:**
  - Check: `rustfmt --check {{files}}`
  - Fix: `rustfmt {{files}}`

#### `cargo_clippy`
- **Files:** Rust projects
- **Features:** Rust linter
- **Commands:**
  - Check: `cargo clippy`
  - Fix: `cargo clippy --fix --allow-dirty --allow-staged`

#### `cargo_check`
- **Files:** Rust projects
- **Features:** Fast type checking
- **Command:** `cargo check`

### Go

#### `go_fmt`
- **Files:** `*.go`
- **Features:** Go formatter
- **Commands:**
  - Check: `gofmt -l {{files}}`
  - Fix: `gofmt -w {{files}}`

#### `go_imports`
- **Files:** `*.go`
- **Features:** Go import management
- **Commands:**
  - Check: `goimports -l {{files}}`
  - Fix: `goimports -w {{files}}`

#### `golangci_lint`
- **Files:** Go projects
- **Features:** Go meta-linter
- **Commands:**
  - Check: `golangci-lint run {{files}}`
  - Fix: `golangci-lint run --fix {{files}}`

#### `staticcheck`
- **Files:** `*.go`
- **Features:** Go static analysis
- **Command:** `staticcheck {{files}}`

#### `go_vet`
- **Files:** Go packages
- **Features:** Go code vetting
- **Command:** `go vet ./...`

#### `gomod_tidy`
- **Files:** `go.mod`
- **Features:** Go module maintenance
- **Command:** `go mod tidy -diff` (with diff support)

#### `revive`
- **Files:** `*.go`
- **Features:** Fast Go linter
- **Command:** `revive {{files}}`

#### `go_lines`
- **Files:** `*.go`
- **Features:** Long line fixer
- **Commands:**
  - Check: `golines --dry-run {{files}}`
  - Fix: `golines -w {{files}}`

#### `go_sec`
- **Files:** Go packages
- **Features:** Security scanner
- **Command:** `gosec ./...`

#### `go_vuln_check`
- **Files:** Go projects
- **Features:** Vulnerability scanner
- **Command:** `govulncheck ./...`

#### `err_check`
- **Files:** Go packages
- **Features:** Error handling checker
- **Command:** `errcheck ./...`

### Ruby

#### `rubocop`
- **Files:** `*.rb`, `*.rake`, `Gemfile`, `Rakefile`
- **Features:** Ruby style guide
- **Commands:**
  - Check: `rubocop {{files}}`
  - Fix: `rubocop -a {{files}}`

#### `standard_rb`
- **Files:** `*.rb`
- **Features:** Ruby Standard Style
- **Commands:**
  - Check: `standardrb {{files}}`
  - Fix: `standardrb --fix {{files}}`

#### `sorbet`
- **Files:** Ruby projects
- **Features:** Type checker for Ruby
- **Command:** `srb tc`

#### `reek`
- **Files:** `*.rb`
- **Features:** Code smell detector
- **Command:** `reek {{files}}`

#### `erb`
- **Files:** `*.erb`
- **Features:** ERB template linter
- **Command:** `erb -x -T - {{files}} | ruby -c`

#### `fasterer`
- **Files:** `*.rb`
- **Features:** Performance suggestions
- **Command:** `fasterer {{files}}`

#### `brakeman`
- **Files:** Rails projects
- **Features:** Security scanner
- **Command:** `brakeman`

#### `bundle_audit`
- **Files:** `Gemfile.lock`
- **Features:** Dependency security audit
- **Commands:**
  - Check: `bundle-audit check`
  - Fix: `bundle-audit update`

### Shell

#### `shellcheck`
- **Files:** `*.sh`, `*.bash`
- **Features:** Shell script analyzer
- **Command:** `shellcheck {{files}}`

#### `shfmt`
- **Files:** `*.sh`, `*.bash`
- **Features:** Shell formatter
- **Commands:**
  - Check: `shfmt -l {{files}}`
  - Fix: `shfmt -w {{files}}`

### Infrastructure

#### `terraform`
- **Files:** `*.tf`, `*.tfvars`
- **Features:** Terraform formatter
- **Commands:**
  - Check: `terraform fmt -check {{files}}`
  - Fix: `terraform fmt {{files}}`

#### `tf_lint`
- **Files:** `*.tf`
- **Features:** Terraform linter
- **Command:** `tflint {{files}}`

#### `hadolint`
- **Files:** `Dockerfile*`
- **Features:** Dockerfile linter
- **Command:** `hadolint {{files}}`

#### `actionlint`
- **Files:** `.github/workflows/*.yml`, `.github/workflows/*.yaml`
- **Features:** GitHub Actions workflow linter
- **Command:** `actionlint {{files}}`

### Nix

#### `nix_fmt`
- **Files:** `*.nix`
- **Features:** Nix formatter
- **Commands:**
  - Check: `nix fmt -- --check {{files}}`
  - Fix: `nix fmt {{files}}`

#### `nixpkgs_format`
- **Files:** `*.nix`
- **Features:** Nixpkgs formatter
- **Commands:**
  - Check: `nixpkgs-fmt --check {{files}}`
  - Fix: `nixpkgs-fmt {{files}}`

#### `alejandra`
- **Files:** `*.nix`
- **Features:** Alternative Nix formatter
- **Commands:**
  - Check: `alejandra --check {{files}}`
  - Fix: `alejandra {{files}}`

### Data Formats

#### `jq`
- **Files:** `*.json`
- **Features:** JSON processor
- **Commands:**
  - Check: `jq empty {{files}}`
  - Fix: `jq . {{files}} | sponge {{files}}`

#### `yq`
- **Files:** `*.yaml`, `*.yml`
- **Features:** YAML processor
- **Commands:**
  - Check: `yq eval '.' {{files}}`
  - Fix: `yq eval '.' -i {{files}}`

#### `yamllint`
- **Files:** `*.yaml`, `*.yml`
- **Features:** YAML linter
- **Command:** `yamllint {{files}}`

#### `xmllint`
- **Files:** `*.xml`
- **Features:** XML validator and formatter
- **Commands:**
  - Check: `xmllint --noout {{files}}`
  - Fix: `xmllint --format {{files}} -o {{files}}`

#### `taplo`
- **Files:** `*.toml`
- **Features:** TOML formatter
- **Commands:**
  - Check: `taplo fmt --check {{files}}`
  - Fix: `taplo fmt {{files}}`

#### `sql_fluff`
- **Files:** `*.sql`
- **Features:** SQL linter and formatter
- **Commands:**
  - Check: `sqlfluff lint {{files}}`
  - Fix: `sqlfluff fix {{files}}`

### Configuration

#### `pkl`
- **Files:** `*.pkl`
- **Features:** Pkl configuration language
- **Command:** `pkl eval {{files}}`

#### `sort_package_json`
- **Files:** `package.json`
- **Features:** Sort package.json keys
- **Commands:**
  - Check: `sort-package-json --check {{files}}`
  - Fix: `sort-package-json {{files}}`

### Markdown

#### `markdown_lint`
- **Files:** `*.md`
- **Features:** Markdown linter
- **Commands:**
  - Check: `markdownlint {{files}}`
  - Fix: `markdownlint --fix {{files}}`

### CSS

#### `stylelint`
- **Files:** `*.css`, `*.scss`, `*.sass`, `*.less`
- **Features:** CSS linter
- **Commands:**
  - Check: `stylelint {{files}}`
  - Fix: `stylelint --fix {{files}}`

### PHP

#### `php_cs`
- **Files:** `*.php`
- **Features:** PHP coding standards fixer
- **Commands:**
  - Check: `php-cs-fixer fix --dry-run {{files}}`
  - Fix: `php-cs-fixer fix {{files}}`

### Other Languages

#### `ktlint`
- **Files:** `*.kt`, `*.kts`
- **Features:** Kotlin linter and formatter
- **Commands:**
  - Check: `ktlint {{files}}`
  - Fix: `ktlint -F {{files}}`

#### `swiftlint`
- **Files:** `*.swift`
- **Features:** Swift style and conventions
- **Commands:**
  - Check: `swiftlint lint {{files}}`
  - Fix: `swiftlint --fix {{files}}`

#### `clang_format`
- **Files:** `*.c`, `*.cpp`, `*.h`, `*.hpp`, `*.cc`, `*.cxx`
- **Features:** C/C++ formatter
- **Commands:**
  - Check: `clang-format --dry-run -Werror {{files}}`
  - Fix: `clang-format -i {{files}}`

#### `cpp_lint`
- **Files:** `*.c`, `*.cpp`, `*.h`, `*.hpp`, `*.cc`, `*.cxx`
- **Features:** C++ style checker
- **Command:** `cpplint {{files}}`

#### `luacheck`
- **Files:** `*.lua`
- **Features:** Lua linter
- **Command:** `luacheck {{files}}`

#### `stylua`
- **Files:** `*.lua`
- **Features:** Lua formatter
- **Commands:**
  - Check: `stylua --check {{files}}`
  - Fix: `stylua {{files}}`

#### `astro`
- **Files:** `*.astro`
- **Features:** Astro component formatter
- **Commands:**
  - Check: `astro check {{files}}`
  - Fix: `astro format {{files}}`

### Special Purpose

#### `check_case_conflict`
- **Files:** All files
- **Features:** Detect case-insensitive filename conflicts
- **Commands:**
  - Check: `hk util check-case-conflict {{files}}`
- **Notes:** Useful for cross-platform projects to avoid conflicts on Windows/macOS

#### `check_executables_have_shebangs`
- **Files:** All files
- **Features:** Verify executable files have shebang lines
- **Commands:**
  - Check: `hk util check-executables-have-shebangs {{files}}`
- **Notes:** Only checks files with execute permission, skips binary files

#### `check_merge_conflict`
- **Files:** All files
- **Features:** Detect merge conflict markers
- **Commands:**
  - Check: `hk util check-merge-conflict {{files}}`
- **Notes:** Detects `<<<<<<<`, `=======`, and `>>>>>>>` markers

#### `check_symlinks`
- **Files:** All files
- **Features:** Detect broken symlinks
- **Commands:**
  - Check: `hk util check-symlinks {{files}}`
- **Notes:** Only flags symlinks that point to non-existent targets

#### `mixed_line_ending`
- **Files:** All text files
- **Features:** Detect and fix mixed line endings (CRLF/LF in same file)
- **Commands:**
  - Check: `hk util mixed-line-ending {{files}}`
  - Fix: `hk util mixed-line-ending --fix {{files}}`
- **Notes:** Normalizes to LF, automatically skips binary files

#### `newlines`
- **Files:** All text files
- **Features:** Ensure files end with newline
- **Commands:**
  - Check: Shell script to check newlines
  - Fix: Shell script to add newlines

#### `trailing_whitespace`
- **Files:** All text files
- **Features:** Detect and remove trailing whitespace from lines
- **Commands:**
  - Check: `hk util trailing-whitespace {{files}}`
  - Fix: `hk util trailing-whitespace --fix {{files}}`
- **Notes:** Uses cross-platform Rust implementation (works on Windows, macOS, Linux)

## Customizing Builtins

### Override Properties

```pkl
["prettier"] = (Builtins.prettier) {
  // Override glob patterns
  glob = List("src/**/*.js", "src/**/*.ts")
  
  // Disable batch processing
  batch = false
  
  // Add environment variables
  env {
    ["PRETTIER_CONFIG"] = ".prettierrc.json"
  }
}
```

### Add Dependencies

```pkl
["eslint"] = (Builtins.eslint) {
  // Run after prettier
  depends = "prettier"
}
```

### Workspace-Specific Configuration

```pkl
["cargo_clippy"] = (Builtins.cargo_clippy) {
  // Only run in directories with Cargo.toml
  workspace_indicator = "Cargo.toml"
  
  // Custom command using workspace
  check = "cargo clippy --manifest-path {{workspace}}/Cargo.toml"
}
```

### Profile-Based Configuration

```pkl
["mypy"] = (Builtins.mypy) {
  // Only run with "python" profile
  profiles = List("python")
}
```

## Creating Custom Steps

If a builtin doesn't exist for your tool:

```pkl
["custom-tool"] {
  glob = List("*.custom")
  check = "custom-tool --check {{files}}"
  fix = "custom-tool --fix {{files}}"
  batch = true  // Enable parallel processing
}
```

## See Also

- [Configuration Schema Reference](reference/schema.md)
- [Configuration Guide](configuration.md)
- [Getting Started](getting_started.md)

#### `check_byte_order_marker`
- **Files:** All files
- **Features:** Detect UTF-8 BOM
- **Commands:** Check: `hk util check-byte-order-marker {{files}}`

#### `fix_byte_order_marker`
- **Files:** All files
- **Features:** Remove UTF-8 BOM
- **Commands:** Fix: `hk util fix-byte-order-marker {{files}}`

#### `check_added_large_files`
- **Files:** All files
- **Features:** Prevent committing large files (default limit: 500KB)
- **Commands:** Check: `hk util check-added-large-files {{files}}`

#### `detect_private_key`
- **Files:** All files
- **Features:** Detect accidentally committed private keys (RSA, OpenSSH, etc.)
- **Commands:** Check: `hk util detect-private-key {{files}}`

#### `no_commit_to_branch`
- **Files:** N/A (git branch check)
- **Features:** Prevent direct commits to protected branches (main, master)
- **Commands:** Check: `hk util no-commit-to-branch`

#### `python_check_ast`
- **Files:** `*.py`
- **Features:** Validate Python syntax by parsing the AST
- **Commands:** Check: `hk util python-check-ast {{files}}`

#### `python_debug_statements`
- **Files:** `*.py`
- **Features:** Detect debug statements (pdb, breakpoint) in Python code
- **Commands:** Check: `hk util python-debug-statements {{files}}`
