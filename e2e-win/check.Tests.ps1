Describe 'check' {
    BeforeAll {
        $script:originalPath = Get-Location
    }

    AfterAll {
        Set-Location $script:originalPath
    }

    It 'runs check with a simple step' {
        $testDir = Join-Path $TestDrive ([System.Guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $testDir | Out-Null
        Set-Location $testDir

        try {
            git init | Out-Null
            git config user.email "test@test.com"
            git config user.name "Test"

            # Create a simple hk.pkl with an echo step
            $config = @"
amends "$env:PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["echo-test"] {
                check = "echo hello"
            }
        }
    }
}
"@
            $config | Out-File -FilePath "hk.pkl" -Encoding utf8

            # Create a dummy file to commit
            "test" | Out-File -FilePath "test.txt" -Encoding utf8
            git add test.txt
            git commit -m "initial" | Out-Null

            $output = hk check 2>&1
            $LASTEXITCODE | Should -Be 0
        } finally {
            Set-Location $script:originalPath
            Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
