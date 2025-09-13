BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk Git Integration" {
    It "Should work with git staged files" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["check-staged"] {
                check = "echo Checking {{files}}"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        "test content" | Out-File -FilePath "test.txt" -Encoding UTF8
        git add test.txt | Out-Null
        git commit -m "initial commit" | Out-Null

        "modified content" | Out-File -FilePath "test.txt" -Encoding UTF8
        git add test.txt | Out-Null

        $output = & $global:HkCommand run pre-commit 2>&1
        $LASTEXITCODE | Should -Be 0
        ($output -join " ") | Should -Match "test.txt"
    }
}