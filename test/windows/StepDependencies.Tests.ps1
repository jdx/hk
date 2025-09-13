BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk Step Dependencies" {
    BeforeEach {
        $script:TestDir = New-TestDirectory
    }

    AfterEach {
        Remove-TestDirectory $script:TestDir
    }

    It "Should respect step dependencies" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["first"] { check = "echo First step" }
            ["second"] {
                check = "echo Second step"
                depends = ["first"]
            }
            ["third"] {
                check = "echo Third step"
                depends = ["second"]
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        $output = & $global:HkCommand check 2>&1
        $LASTEXITCODE | Should -Be 0
        ($output -join " ") | Should -Match "(First step|first)"
        ($output -join " ") | Should -Match "(Second step|second)"
        ($output -join " ") | Should -Match "(Third step|third)"
    }
}
