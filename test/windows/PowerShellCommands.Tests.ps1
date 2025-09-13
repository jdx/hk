BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk PowerShell Command Execution" {
    It "Should execute PowerShell commands" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ps-test"] {
                check = "Write-Host 'PowerShell test successful'"
                shell = "powershell.exe -NoProfile -Command"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        $output = & $global:HkCommand check 2>&1
        $LASTEXITCODE | Should -Be 0
        ($output -join " ") | Should -Match "(PowerShell test successful|ps-test)"
    }

    It "Should execute pwsh commands if available" {
        if (-not (Get-Command pwsh.exe -ErrorAction SilentlyContinue)) {
            Set-ItResult -Skipped -Because "pwsh.exe not available"
            return
        }

        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["pwsh-test"] {
                check = "Write-Host 'PowerShell Core test successful'"
                shell = "pwsh.exe -NoProfile -Command"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        $output = & $global:HkCommand check 2>&1
        $LASTEXITCODE | Should -Be 0
        ($output -join " ") | Should -Match "(PowerShell Core test successful|pwsh-test)"
    }
}
