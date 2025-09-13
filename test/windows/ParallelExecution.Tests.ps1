BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk Parallel Execution" {
    It "Should run multiple steps in parallel" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] { check = "echo Step 1 completed" }
            ["step2"] { check = "echo Step 2 completed" }
            ["step3"] { check = "echo Step 3 completed" }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        $output = & $global:HkCommand check 2>&1
        $LASTEXITCODE | Should -Be 0
        ($output -join " ") | Should -Match "(Step 1 completed|step1)"
        ($output -join " ") | Should -Match "(Step 2 completed|step2)"
        ($output -join " ") | Should -Match "(Step 3 completed|step3)"
    }
}