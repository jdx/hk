# Common test setup and utilities for hk Windows tests
# This file is sourced by each test file to set up the test environment

# Setup test environment
if (-not $global:TestRoot) {
    $global:TestRoot = Join-Path $env:TEMP ("hk-test-" + [System.Guid]::NewGuid().ToString())
    New-Item -Path $global:TestRoot -ItemType Directory -Force | Out-Null
}

# Always find the local hk.exe binary (try debug first, then release)
if (-not $global:HkPath) {
    # Try from test/windows directory
    $global:HkPath = Resolve-Path "..\..\target\debug\hk.exe" -ErrorAction SilentlyContinue
    if (-not $global:HkPath) {
        $global:HkPath = Resolve-Path "..\..\target\release\hk.exe" -ErrorAction SilentlyContinue
    }
    # Try from project root
    if (-not $global:HkPath) {
        $global:HkPath = Resolve-Path "target\debug\hk.exe" -ErrorAction SilentlyContinue
    }
    if (-not $global:HkPath) {
        $global:HkPath = Resolve-Path "target\release\hk.exe" -ErrorAction SilentlyContinue
    }
    if (-not $global:HkPath) {
        throw "Could not find hk.exe. Please build the project first."
    }
}

# Determine how to run hk commands
if (-not $global:HkCommand) {
    # Check if pkl is available in PATH
    $pklAvailable = Get-Command pkl -ErrorAction SilentlyContinue

    if ($env:USE_MISE -eq "true" -or -not $pklAvailable) {
        # Use mise to provide PKL when not in PATH
        Write-Host "Using mise to provide PKL for hk" -ForegroundColor Gray
        $global:HkCommand = {
            param([Parameter(ValueFromRemainingArguments=$true)]$CommandArgs)
            & mise x -- $global:HkPath $CommandArgs
        }
    } else {
        # PKL is in PATH, use direct path to hk.exe
        Write-Host "PKL found in PATH, running hk directly" -ForegroundColor Gray
        $global:HkCommand = {
            param([Parameter(ValueFromRemainingArguments=$true)]$CommandArgs)
            & $global:HkPath $CommandArgs
        }
    }
}

# Register cleanup for when all tests are done
if (-not $global:CleanupRegistered) {
    $global:CleanupRegistered = $true

    # This will be called by the main test runner
    $global:TestCleanup = {
        if (Test-Path $global:TestRoot) {
            Remove-Item $global:TestRoot -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# Function to create a test directory for each test
function New-TestDirectory {
    $testDir = New-Item -Path (Join-Path $global:TestRoot ([System.Guid]::NewGuid().ToString())) -ItemType Directory
    Push-Location $testDir

    # Initialize git repository
    git init | Out-Null
    git config user.email "test@example.com" | Out-Null
    git config user.name "Test User" | Out-Null

    return $testDir
}

# Function to clean up a test directory
function Remove-TestDirectory {
    param($TestDir)

    Pop-Location
    if (Test-Path $TestDir) {
        Remove-Item $TestDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Functions are already global and accessible