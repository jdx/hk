BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk Error Handling" {
    It "Should handle failing commands" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["failing-step"] {
                check = "exit 1"
                shell = "cmd.exe /C"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        & $global:HkCommand check 2>&1 | Out-Null
        $LASTEXITCODE | Should -Not -Be 0
    }

    It "Should continue with other steps when fail-fast is disabled" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        fail_fast = false
        steps {
            ["failing"] { check = "exit 1" shell = "cmd.exe /C" }
            ["passing"] { check = "echo Success" }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        $output = & $global:HkCommand check 2>&1
        # Should still fail overall but run both steps
        $LASTEXITCODE | Should -Not -Be 0
        ($output -join " ") | Should -Match "(Success|passing)"
    }
}
