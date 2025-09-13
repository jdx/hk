# Windows Integration Tests

This directory contains Windows-specific integration tests for hk using PowerShell and Pester.

## Overview

Since the main test suite uses bats (Bash Automated Testing System) which is Unix-specific, we provide a separate test suite for Windows using Pester, PowerShell's native testing framework.

## Requirements

- Windows 10 or later
- PowerShell 5.1 or PowerShell Core 6+
- Git
- Rust and Cargo (for building hk)
- Pester module (automatically installed if missing)

## Running Tests

### Quick Start

```powershell
# Build and run all tests
.\run-tests.ps1
```

### Manual Execution

```powershell
# Build hk first
cargo build --release

# Install Pester if not already installed
Install-Module -Name Pester -Force -SkipPublisherCheck

# Run tests
Invoke-Pester -Path "hk.Tests.ps1" -Output Detailed
```

### CI/CD

The tests are automatically run in GitHub Actions on Windows runners. See `.github/workflows/ci-windows.yml` for the full configuration.

## Test Coverage

The Windows tests cover:

### Basic Functionality
- `hk init` - Configuration initialization
- `hk validate` - Configuration validation
- `hk install/uninstall` - Git hook management

### Windows-Specific Features
- PowerShell command execution
- CMD command execution
- Windows path handling
- Batch file generation for git hooks

### Core Features
- Parallel step execution
- Step dependencies
- File glob patterns
- Git integration
- Error handling
- Configuration formats

### Shell Compatibility
- PowerShell (`powershell.exe`)
- PowerShell Core (`pwsh.exe`) when available
- Command Prompt (`cmd.exe`)

## Test Structure

- `hk.Tests.ps1` - Main test file containing all Windows integration tests
- `run-tests.ps1` - Test runner script with options
- `README.md` - This documentation

## Differences from Unix Tests

The Windows tests focus on Windows-specific functionality that differs from the Unix bats tests:

1. **Shell Execution**: Tests PowerShell and CMD instead of bash/sh
2. **Path Handling**: Tests Windows path separators and conventions
3. **Hook Generation**: Validates Windows batch files instead of shell scripts
4. **Tool Detection**: Tests Windows-specific tool discovery

## Adding New Tests

When adding new tests, follow the Pester conventions:

```powershell
Context "Feature Name" {
    It "Should do something specific" {
        # Arrange
        # Setup test data
        
        # Act
        # Execute the command
        
        # Assert
        # Verify the results
        $result | Should -Be $expected
    }
}
```

## Troubleshooting

### Common Issues

1. **Pester not found**: Install with `Install-Module -Name Pester -Force`
2. **hk.exe not found**: Build with `cargo build --release`
3. **Git not configured**: Tests automatically configure git for testing
4. **Permission errors**: Run PowerShell as Administrator if needed

### Debugging

To debug test failures:

```powershell
# Run a specific test
Invoke-Pester -Path "hk.Tests.ps1" -TestName "*Should initialize hk configuration*"

# Run with verbose output
Invoke-Pester -Path "hk.Tests.ps1" -Output Detailed -Verbose
```

## Contributing

When contributing Windows-specific functionality:

1. Add corresponding tests to `hk.Tests.ps1`
2. Ensure tests work on both PowerShell 5.1 and PowerShell Core
3. Test both `powershell.exe` and `pwsh.exe` paths when available
4. Update this README if new test categories are added