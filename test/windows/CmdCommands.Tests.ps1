BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk CMD Command Execution" {
    It "Should execute CMD commands" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["cmd-test"] {
                check = "echo CMD test successful"
                shell = "cmd.exe /C"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        $output = & $global:HkCommand check 2>&1
        $LASTEXITCODE | Should -Be 0
        ($output -join " ") | Should -Match "(CMD test successful|cmd-test)"
    }
}
