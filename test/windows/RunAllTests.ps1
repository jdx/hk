#!/usr/bin/env pwsh
# Main test runner for hk Windows tests

param(
    [string]$TestName = "*",
    [string]$Output = "Detailed"
)

# Ensure Pester is installed
if (-not (Get-Module -ListAvailable -Name Pester)) {
    Write-Host "Installing Pester module..." -ForegroundColor Yellow
    Install-Module -Name Pester -Force -SkipPublisherCheck
}

Import-Module Pester

# Define test files (in the order they should run)
$testFiles = @(
    "BasicCommands.Tests.ps1",
    "HookInstallation.Tests.ps1",
    "PowerShellCommands.Tests.ps1",
    "CmdCommands.Tests.ps1",
    "FileHandling.Tests.ps1",
    "ParallelExecution.Tests.ps1",
    "StepDependencies.Tests.ps1",
    "ErrorHandling.Tests.ps1",
    "GitIntegration.Tests.ps1",
    "ConfigurationFormats.Tests.ps1"
)

# Filter test files based on pattern
if ($TestName -ne "*") {
    $testFiles = $testFiles | Where-Object { $_ -like "*$TestName*" }
}

# Build full paths
$testPaths = $testFiles | ForEach-Object {
    Join-Path $PSScriptRoot $_
}

# Run tests
Write-Host "Running hk Windows tests..." -ForegroundColor Cyan
Write-Host "Test files: $($testFiles -join ', ')" -ForegroundColor Gray

$config = New-PesterConfiguration
$config.Run.Path = $testPaths
$config.Output.Verbosity = $Output
$config.Run.Exit = $false

$result = Invoke-Pester -Configuration $config

# Summary
Write-Host ""
Write-Host "Test Summary:" -ForegroundColor Cyan
Write-Host "  Total: $($result.TotalCount)" -ForegroundColor Gray
Write-Host "  Passed: $($result.PassedCount)" -ForegroundColor Green
Write-Host "  Failed: $($result.FailedCount)" -ForegroundColor Red
Write-Host "  Skipped: $($result.SkippedCount)" -ForegroundColor Yellow

# Exit with appropriate code
exit $result.FailedCount
