# Windows Support Implementation

This document outlines the changes made to add Windows support to hk, resolving [GitHub discussion #225](https://github.com/jdx/hk/discussions/225).

## Overview

The hk project previously had Unix-specific dependencies that prevented it from working natively on Windows. This implementation adds Windows compatibility while maintaining existing Unix functionality.

## Changes Made

### 1. Windows-Compatible Shell Execution

**New File: `src/shell.rs`**
- Added a new shell abstraction layer with cross-platform support
- Detects available shells on Windows (PowerShell, PowerShell Core, CMD)
- Provides shell-specific command execution and quoting
- Falls back gracefully based on available tools

**Key Features:**
- Auto-detection of PowerShell (`powershell.exe`) or PowerShell Core (`pwsh.exe`)
- Fallback to Command Prompt (`cmd.exe`)
- Proper command quoting for each shell type
- Platform-specific shebang and file extension support

### 2. Updated Core Modules

**Modified Files:**
- `src/main.rs` - Added shell module import
- `src/step.rs` - Uses new shell detection for command execution  
- `src/hook.rs` - Updated report command execution for Windows
- `src/test_runner.rs` - Cross-platform test command execution
- `src/git.rs` - Windows-compatible git command execution
- `src/config.rs` - Windows path handling for pkl config parsing

**Changes:**
- Replaced hard-coded `sh -o errexit -c` calls with platform-aware shell detection
- Added Windows-specific command execution paths
- Proper handling of Windows paths and separators
- Cross-platform error handling

### 3. Git Hook Installation

**Modified File: `src/cli/install.rs`**
- Windows hooks generate as batch files (`.bat`) instead of shell scripts
- Unix hooks continue to use shell scripts
- Platform-specific shebang and syntax
- Conditional executable permission setting (Unix only)

**Windows Hook Format:**
```batch
@echo off
if "%HK%"=="0" (
    exit /b 0
) else (
    hk run pre-commit %*
)
```

**Unix Hook Format:**
```bash
#!/bin/sh
test "${HK:-1}" = "0" || exec hk run pre-commit "$@"
```

### 4. Windows-Specific Testing

**New Files:**
- `test/windows/hk.Tests.ps1` - Comprehensive Pester test suite
- `test/windows/run-tests.ps1` - Test runner script
- `test/windows/README.md` - Windows testing documentation

**Test Coverage:**
- Basic commands (init, validate, install, uninstall)
- PowerShell and CMD command execution
- Windows path handling
- Parallel execution
- Step dependencies
- File glob patterns
- Git integration
- Error handling

### 5. CI/CD Integration

**Modified Files:**
- `.github/workflows/ci.yml` - Enabled Windows in matrix
- `.github/workflows/ci-windows.yml` - Dedicated Windows workflow

**CI Features:**
- Automated Windows testing on GitHub Actions
- Pester test execution
- pkl installation for Windows
- Build artifact generation for releases
- Both PowerShell and CMD testing

## Technical Details

### Shell Detection Logic

```rust
pub fn detect() -> Self {
    if cfg!(windows) {
        if which::which("powershell.exe").is_ok() || which::which("pwsh.exe").is_ok() {
            Shell::PowerShell
        } else {
            Shell::Cmd
        }
    } else {
        // Unix shell detection logic
    }
}
```

### Command Execution

The implementation provides three execution paths:

1. **PowerShell/PowerShell Core**: `powershell.exe -NoProfile -NonInteractive -Command`
2. **Command Prompt**: `cmd.exe /C`
3. **Unix shells**: `sh -o errexit -c` (unchanged)

### Path Handling

Windows paths are properly handled in:
- Configuration file parsing with pkl
- Git command execution
- File glob pattern matching
- Test fixture creation

## Compatibility

### Windows Support
- **Windows 10+**: Full support
- **PowerShell 5.1+**: Recommended
- **PowerShell Core 6+**: Enhanced support with `pwsh.exe`
- **Command Prompt**: Basic support

### Unix Support
- **Maintained**: All existing Unix functionality preserved
- **No Breaking Changes**: Existing Unix workflows unchanged
- **Shell Support**: bash, zsh, fish, dash, sh

## Testing

### Windows Testing (Pester) 
```powershell
# Set required environment variable for mise/pkl
$env:MISE_DISABLE_TOOLS="hadolint,swiftlint,bun"

# Run all Windows tests
cd test/windows
./run-tests.ps1
```

### Verified Functionality ✅
- **hk init**: Creates pkl configuration files
- **hk validate**: Validates pkl configurations 
- **hk check**: Executes PowerShell and CMD commands
- **hk install**: Creates Windows batch file hooks
- **Cross-platform shell detection**: Auto-detects available shells

### Unix Testing (bats)
```bash
# Run existing Unix tests (unchanged)
mise run test:bats
```

## Usage Examples

### Windows PowerShell
```powershell
# Initialize hk
hk init

# Install hooks
hk install

# Run checks with PowerShell commands
# hk.pkl:
hooks {
    ["check"] {
        steps {
            ["ps-lint"] {
                check = "Write-Host 'Linting PowerShell files'"
                glob = ["*.ps1"]
            }
        }
    }
}
```

### Windows CMD
```cmd
# hk.pkl with CMD commands:
hooks {
    ["check"] {
        steps {
            ["cmd-check"] {
                check = "echo Checking files"
                shell = "cmd.exe /C"
            }
        }
    }
}
```

## Migration Guide

### For Windows Users
1. **No Action Required**: Windows users can now use hk directly
2. **PowerShell Recommended**: Install PowerShell 5.1+ for best experience
3. **pkl Installation**: Install pkl for Windows from [Apple's releases](https://github.com/apple/pkl/releases)

### For Existing Unix Users
- **No Changes Required**: All existing configurations work unchanged
- **New Windows Contributors**: Can now contribute and test on Windows

## Future Enhancements

### Potential Improvements
- Windows-specific builtin linters (e.g., PSScriptAnalyzer)
- Enhanced Windows path handling for UNC paths
- Integration with Windows Subsystem for Linux (WSL)
- PowerShell DSC integration possibilities

### Known Limitations
- Some Unix-specific tools may not be available on Windows
- Performance characteristics may differ between platforms
- PowerShell execution policy restrictions may apply

## Dependencies

### New Dependencies
- `which` crate: Cross-platform executable detection (already present)
- `eyre` crate: Error handling (already present)

### Windows-Specific Requirements
- Windows 10 or later
- PowerShell 5.1+ (recommended) or Command Prompt
- Git for Windows
- pkl for Windows

## Files Changed

### Core Implementation
- `src/shell.rs` (new)
- `src/main.rs`
- `src/step.rs` 
- `src/hook.rs`
- `src/test_runner.rs`
- `src/git.rs`
- `src/config.rs`
- `src/cli/install.rs`

### Testing
- `test/windows/hk.Tests.ps1` (new)
- `test/windows/run-tests.ps1` (new)
- `test/windows/README.md` (new)

### CI/CD
- `.github/workflows/ci.yml`
- `.github/workflows/ci-windows.yml` (new)

### Documentation
- `WINDOWS_SUPPORT.md` (this file, new)

## Conclusion

This implementation successfully resolves the Windows compatibility issues identified in GitHub discussion #225 by:

1. **Adding cross-platform shell abstraction** that handles Windows PowerShell, CMD, and Unix shells
2. **Implementing Windows-specific git hook generation** with proper batch file syntax
3. **Creating comprehensive Windows test suite** using Pester instead of Unix-specific bats
4. **Integrating Windows testing into CI/CD** pipelines for continuous validation
5. **Maintaining backward compatibility** with all existing Unix functionality

Windows users can now use hk with the same feature set as Unix users, while the codebase remains maintainable with clear platform-specific abstractions.