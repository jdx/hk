Describe 'validate' {
    BeforeAll {
        $script:originalPath = Get-Location
    }

    AfterAll {
        Set-Location $script:originalPath
    }

    It 'validates a valid hk.pkl config' {
        $testDir = Join-Path $TestDrive ([System.Guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $testDir | Out-Null
        Set-Location $testDir

        try {
            git init | Out-Null
            git config user.email "test@test.com"
            git config user.name "Test"

            hk init
            $output = hk validate 2>&1
            $LASTEXITCODE | Should -Be 0
        } finally {
            Set-Location $script:originalPath
            Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
