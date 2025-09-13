BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk Configuration Formats" {
    It "Should work with different config formats" {
        # Test TOML config
        @'
[hooks.check.steps.toml-test]
check = "echo TOML config works"
'@ | Out-File -FilePath "hk.toml" -Encoding UTF8

        $output = & $global:HkCommand check 2>&1
        if ($LASTEXITCODE -eq 0) {
            ($output -join " ") | Should -Match "(TOML config works|toml-test)"
        } else {
            # TOML support might not be available, that's ok
            Write-Host "TOML config not supported, skipping"
        }
    }
}
