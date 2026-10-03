Describe 'fix' {
    BeforeAll {
        $script:originalPath = Get-Location

        function New-TestRepo {
            param([string]$Hooks)
            $dir = Join-Path $TestDrive ([System.Guid]::NewGuid().ToString())
            New-Item -ItemType Directory -Path $dir | Out-Null
            Set-Location $dir
            git init | Out-Null
            git config user.email 'test@test.com'
            git config user.name 'Test'
            git config core.autocrlf false
            git config commit.gpgsign false
            $pklPath = (Resolve-Path $env:PKL_PATH).Path -replace '\\', '/'
            $config = "amends `"file:///$pklPath/Config.pkl`"`n$Hooks"
            [System.IO.File]::WriteAllText("$dir/hk.pkl", $config)
            return $dir
        }

        function Write-Lf {
            param([string]$Path, [string]$Content)
            [System.IO.File]::WriteAllText((Join-Path (Get-Location) $Path), $Content)
        }

        function Read-Lf {
            param([string]$Path)
            [System.IO.File]::ReadAllText((Join-Path (Get-Location) $Path))
        }

        # Fixers built from hk itself so the tests need no extra tools.
        $script:whitespaceHooks = @'
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["whitespace"] {
                glob = "*.txt"
                fix = "hk util trailing-whitespace --fix {{files}}"
            }
        }
    }
    ["fix"] {
        steps {
            ["whitespace"] {
                glob = "*.txt"
                fix = "hk util trailing-whitespace --fix {{files}}"
            }
            ["eof"] {
                glob = "*.txt"
                fix = "hk util end-of-file-fixer --fix {{files}}"
            }
        }
    }
}
'@
    }

    AfterEach {
        Set-Location $script:originalPath
    }

    It 'fix --all rewrites files and overlapping fixers both apply' {
        $dir = New-TestRepo $script:whitespaceHooks
        Write-Lf 'a.txt' "one   `ntwo"
        git add -A
        git commit -m initial | Out-Null

        $output = hk fix --all 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because "hk fix --all should succeed: $output"
        # trailing-whitespace and end-of-file-fixer both rewrite a.txt; the
        # file locks must serialize them so neither fix is lost.
        Read-Lf 'a.txt' | Should -BeExactly "one`ntwo`n"
    }

    It 'pre-commit fix re-stages fixed files' {
        $dir = New-TestRepo $script:whitespaceHooks
        Write-Lf 'a.txt' "base`n"
        git add -A
        git commit -m initial | Out-Null

        Write-Lf 'a.txt' "base   `nmore  `n"
        git add a.txt

        $output = hk run pre-commit 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because "hook should succeed: $output"
        Read-Lf 'a.txt' | Should -BeExactly "base`nmore`n"
        (git show ':a.txt' | Out-String).Replace("`r`n", "`n") | Should -BeExactly "base`nmore`n"
        (git status --porcelain | Out-String).Trim() | Should -BeExactly 'M  a.txt'
    }

    It 'pre-commit with a partially staged file keeps unstaged changes' {
        $dir = New-TestRepo $script:whitespaceHooks
        Write-Lf 'a.txt' "base`n"
        git add -A
        git commit -m initial | Out-Null

        Write-Lf 'a.txt' "base`nstaged   `n"
        git add a.txt
        Write-Lf 'a.txt' "base`nstaged   `nunstaged   `n"

        $output = hk run pre-commit 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because "hook should succeed: $output"
        # The index gets the fixed staged content only.
        (git show ':a.txt' | Out-String).Replace("`r`n", "`n") | Should -BeExactly "base`nstaged`n"
        # The working tree keeps the unstaged edit on top of the fix.
        $worktree = Read-Lf 'a.txt'
        $worktree | Should -Match 'unstaged'
        $worktree | Should -Match 'staged'
        (git stash list | Out-String) | Should -BeNullOrEmpty
        (git status --porcelain | Out-String).Trim() | Should -BeExactly 'MM a.txt'
    }

    It 'a real commit runs the installed hook and commits fixed content' {
        $dir = New-TestRepo $script:whitespaceHooks
        Write-Lf 'a.txt' "base`n"
        git add -A
        git commit -m initial | Out-Null
        hk install 2>&1 | Out-Null
        $LASTEXITCODE | Should -Be 0

        Write-Lf 'a.txt' "base   `nnew  `n"
        git add a.txt
        $output = git commit -m 'through hook' 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because "commit should succeed: $output"
        (git show 'HEAD:a.txt' | Out-String).Replace("`r`n", "`n") | Should -BeExactly "base`nnew`n"
        Read-Lf 'a.txt' | Should -BeExactly "base`nnew`n"
        (git log -1 --format=%s | Out-String).Trim() | Should -BeExactly 'through hook'
        (git status --porcelain | Out-String) | Should -BeNullOrEmpty
    }
}
