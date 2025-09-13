#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Run hk Windows integration tests using Pester

.DESCRIPTION
    This script runs the hk Windows integration tests using Pester. It builds the project
    and then runs all Pester tests in the test/windows directory.

.PARAMETER BuildFirst
    Whether to build the project before running tests (default: true)

.PARAMETER TestPath
    Path to the test files (default: test/windows/*.Tests.ps1)

.EXAMPLE
    .\run-tests.ps1
    
.EXAMPLE
    .\run-tests.ps1 -BuildFirst:$false -TestPath "hk.Tests.ps1"
#>

param(
    [bool]$BuildFirst = $true,
    [string]$TestPath = "*.Tests.ps1"
)

# Set error action preference
$ErrorActionPreference = "Stop"

Write-Host "Starting hk Windows integration tests..." -ForegroundColor Green

# Build the project if requested
if ($BuildFirst) {
    Write-Host "Building hk in release mode..." -ForegroundColor Yellow
    if (Get-Command mise -ErrorAction SilentlyContinue) {
        mise run build
    } else {
        cargo build --release
    }
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to build hk"
        exit 1
    }
    Write-Host "Build completed successfully" -ForegroundColor Green
}

# Check if Pester is installed
if (-not (Get-Module -ListAvailable -Name Pester)) {
    Write-Host "Installing Pester..." -ForegroundColor Yellow
    Install-Module -Name Pester -Force -SkipPublisherCheck
}

# Import Pester
Import-Module Pester

# Get the directory where this script is located
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path

# Run the tests
Write-Host "Running Pester tests..." -ForegroundColor Yellow
$TestResults = Invoke-Pester -Path (Join-Path $ScriptDir $TestPath) -Output Detailed -PassThru

# Report results
Write-Host "`nTest Results:" -ForegroundColor Green
Write-Host "  Tests run: $($TestResults.TotalCount)" -ForegroundColor White
Write-Host "  Passed: $($TestResults.PassedCount)" -ForegroundColor Green
Write-Host "  Failed: $($TestResults.FailedCount)" -ForegroundColor Red
Write-Host "  Skipped: $($TestResults.SkippedCount)" -ForegroundColor Yellow

if ($TestResults.FailedCount -gt 0) {
    Write-Host "`nFailed tests:" -ForegroundColor Red
    foreach ($failed in $TestResults.Failed) {
        Write-Host "  - $($failed.FullName)" -ForegroundColor Red
        if ($failed.ErrorRecord) {
            Write-Host "    Error: $($failed.ErrorRecord.Exception.Message)" -ForegroundColor Red
        }
    }
    exit 1
} else {
    Write-Host "`nAll tests passed! ✅" -ForegroundColor Green
    exit 0
}
