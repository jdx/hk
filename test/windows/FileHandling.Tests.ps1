BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk File Handling" {
    BeforeEach {
        $script:TestDir = New-TestDirectory
    }

    AfterEach {
        Remove-TestDirectory $script:TestDir
    }

    It "Should process files with glob patterns" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ps1-files"] {
                check = "Write-Host 'Processing PowerShell files'"
                glob = ["*.ps1"]
            }
            ["txt-files"] {
                check = "Write-Host 'Processing text files'"
                glob = ["*.txt"]
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        "Write-Host 'test'" | Out-File -FilePath "test.ps1" -Encoding UTF8
        "test content" | Out-File -FilePath "test.txt" -Encoding UTF8
        "other content" | Out-File -FilePath "test.log" -Encoding UTF8

        $output = & $global:HkCommand check test.ps1 test.txt 2>&1
        $LASTEXITCODE | Should -Be 0
        ($output -join " ") | Should -Match "(Processing PowerShell files|ps1-files)"
        ($output -join " ") | Should -Match "(Processing text files|txt-files)"
    }

    It "Should handle Windows paths correctly" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["path-test"] {
                check = "echo Processing {{files}}"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        New-Item -Path "subdir" -ItemType Directory | Out-Null
        "content" | Out-File -FilePath "subdir\test.txt" -Encoding UTF8

        $output = & $global:HkCommand check "subdir\test.txt" 2>&1
        $LASTEXITCODE | Should -Be 0
        ($output -join " ") | Should -Match "subdir"
    }
}
