Describe 'stash line endings' {
    BeforeAll {
        $script:originalPath = Get-Location
        $script:originalLibgit2 = $env:HK_LIBGIT2
    }

    AfterEach {
        Set-Location $script:originalPath
        $env:HK_LIBGIT2 = $script:originalLibgit2
    }

    It 'preserves CRLF with <Source> and libgit2=<Backend>' -ForEach @(
        @{ Source = 'attributes'; Backend = '0' }
        @{ Source = 'attributes'; Backend = '1' }
        @{ Source = 'autocrlf'; Backend = '0' }
        @{ Source = 'autocrlf'; Backend = '1' }
    ) {
        $testDir = Join-Path $TestDrive ([System.Guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $testDir | Out-Null
        Set-Location $testDir
        $env:HK_LIBGIT2 = $Backend
        git init | Out-Null
        git config user.email 'test@test.com'
        git config user.name 'Test'
        git config core.autocrlf ($Source -eq 'autocrlf').ToString().ToLowerInvariant()
        $attributes = "lf.txt text eol=lf`n"
        if ($Source -eq 'attributes') {
            $attributes += "crlf.txt text eol=crlf`n"
        }
        [System.IO.File]::WriteAllText("$testDir/.gitattributes", $attributes)
        $pklPath = (Resolve-Path $env:PKL_PATH).Path -replace '\\', '/'
        $config = @"
amends "file:///$pklPath/Config.pkl"
hooks {
    ["pre-commit"] {
        stash = "git"
        steps {
            ["noop"] { check = "echo ok" }
        }
    }
}
"@
        [System.IO.File]::WriteAllText("$testDir/hk.pkl", $config)
        [System.IO.File]::WriteAllText("$testDir/crlf.txt", "one`r`n")
        [System.IO.File]::WriteAllText("$testDir/lf.txt", "one`n")
        git add -A
        git commit -m initial | Out-Null
        $LASTEXITCODE | Should -Be 0
        [System.IO.File]::WriteAllText("$testDir/crlf.txt", "two`r`n")
        [System.IO.File]::WriteAllText("$testDir/lf.txt", "two`n")
        git add lf.txt
        $indexBefore = git write-tree

        $output = hk run pre-commit 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because "hook should succeed: $output"
        [System.IO.File]::ReadAllText("$testDir/crlf.txt") | Should -BeExactly "two`r`n"
        [System.IO.File]::ReadAllText("$testDir/lf.txt") | Should -BeExactly "two`n"
        (git write-tree) | Should -BeExactly $indexBefore
        (git stash list | Out-String) | Should -BeNullOrEmpty
    }
}
