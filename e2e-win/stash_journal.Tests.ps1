Describe 'stash journal' {
    BeforeAll {
        $script:originalPath = Get-Location
    }

    AfterAll {
        Set-Location $script:originalPath
    }

    It 'restores the stash left by a killed hk on the next run' {
        # Outside TestDrive: Pester fails the whole file if the killed run's leftovers still hold the directory
        $testDir = Join-Path ([System.IO.Path]::GetTempPath()) ('hk-sj-repo-' + [System.Guid]::NewGuid())
        New-Item -ItemType Directory -Path $testDir | Out-Null
        Set-Location $testDir

        try {
            git init | Out-Null
            git config user.email "test@test.com"
            git config user.name "Test"
            git config core.autocrlf false

            "base`n" | Out-File -FilePath "file.txt" -Encoding ascii -NoNewline
            $pklPath = (Resolve-Path $env:PKL_PATH).Path -replace '\\', '/'
            $pklUri = "file:///$pklPath/Config.pkl"
            $config = @"
amends "$pklUri"

hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["slow"] {
                glob = "**/*.txt"
                fix = "echo x > .git\\started & ping -n 60 127.0.0.1 >nul"
            }
        }
    }
    ["check"] {
        steps {
            ["quick"] {
                glob = "**/*.txt"
                check = "echo ok"
            }
        }
    }
}
"@
            Set-Content -Path "hk.pkl" -Value $config -Encoding ascii
            git add -A
            git commit -m "initial" | Out-Null
            "staged`n" | Out-File -FilePath "file.txt" -Encoding ascii -NoNewline
            git add file.txt
            "staged`nunstaged`n" | Out-File -FilePath "file.txt" -Encoding ascii -NoNewline
            $gitDir = (git rev-parse --absolute-git-dir).Trim()
            $journal = Join-Path $gitDir 'hk-pending-stash'

            # Output goes outside the repository: untracked files in it would be stashed, or
            # keep the worktree from counting as clean when the next run recovers
            $outDir = Join-Path ([System.IO.Path]::GetTempPath()) ('hk-sj-' + [System.Guid]::NewGuid())
            New-Item -ItemType Directory -Path $outDir | Out-Null
            $hk = Start-Process -FilePath hk -ArgumentList 'run', 'pre-commit' -PassThru -NoNewWindow `
                -RedirectStandardOutput (Join-Path $outDir 'hk-out.txt') -RedirectStandardError (Join-Path $outDir 'hk-err.txt')
            for ($i = 0; $i -lt 200 -and -not (Test-Path (Join-Path $gitDir 'started')); $i++) {
                Start-Sleep -Milliseconds 100
            }
            Test-Path (Join-Path $gitDir 'started') | Should -BeTrue -Because "the step should have started"
            Test-Path $journal | Should -BeTrue -Because "the journal is written before the worktree changes"
            (Get-Content file.txt -Raw).Trim() | Should -Be 'staged'

            # TerminateProcess leaves hk no chance to restore anything
            Stop-Process -Id $hk.Id -Force
            Get-CimInstance Win32_Process |
                Where-Object { $_.CommandLine -like '*ping -n 60*' } |
                ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
            Start-Sleep -Seconds 1
            Remove-Item (Join-Path $gitDir 'started') -ErrorAction SilentlyContinue
            Test-Path $journal | Should -BeTrue
            (git stash list | Measure-Object).Count | Should -Be 1

            cmd /c "hk check --all > `"$outDir\hk-recover.txt`" 2>&1"
            $LASTEXITCODE | Should -Be 0
            ((Get-Content file.txt -Raw) -replace "`r", '') | Should -Be "staged`nunstaged`n"
            (git stash list | Measure-Object).Count | Should -Be 0
            Test-Path $journal | Should -BeFalse
        } finally {
            Get-CimInstance Win32_Process |
                Where-Object { $_.CommandLine -like '*ping -n 60*' } |
                ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
            Set-Location $script:originalPath
            for ($i = 0; $i -lt 10 -and (Test-Path $testDir); $i++) {
                Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue
                if (Test-Path $testDir) { Start-Sleep -Milliseconds 500 }
            }
            if ($outDir) { Remove-Item -Path $outDir -Recurse -Force -ErrorAction SilentlyContinue }
        }
    }
}
