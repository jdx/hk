Describe 'step shell' {
    BeforeAll {
        $script:originalPath = Get-Location
        # Git for Windows ships a POSIX sh under its install directory, which has
        # a space in its path.
        $gitCmd = (Get-Command git -ErrorAction SilentlyContinue).Source
        $script:gitSh = $null
        if ($gitCmd) {
            $candidate = Join-Path (Split-Path (Split-Path $gitCmd)) 'usr\bin\sh.exe'
            if (Test-Path $candidate) { $script:gitSh = $candidate }
        }
        $script:WriteShellConfig = {
            param([string]$shell, [string]$script)
            $pklPath = (Resolve-Path $env:PKL_PATH).Path -replace '\\', '/'
            $config = @"
amends "file:///$pklPath/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["s"] {
                shell = #"$shell"#
                check = #"""
$script
"""#
            }
        }
    }
}
"@
            Set-Content -Path "hk.pkl" -Value $config -Encoding ascii
        }
    }

    AfterAll {
        Set-Location $script:originalPath
    }

    BeforeEach {
        $script:testDir = Join-Path $TestDrive ([System.Guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $script:testDir | Out-Null
        Set-Location $script:testDir
        git init | Out-Null
        git config user.email "test@test.com"
        git config user.name "Test"
        "test" | Out-File -FilePath "test.txt" -Encoding ascii
        git add -A
        git commit -m "initial" | Out-Null
    }

    AfterEach {
        Set-Location $script:originalPath
        Remove-Item -Path $script:testDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    It 'runs a multi-line script to the end and reports its exit code' {
        # The script used to be handed to `cmd.exe /c`, which stops at the first
        # newline, so only `echo` ran and hk reported success. Prefer a bare `sh`,
        # which hk could always split, so this case is about the wrapper alone.
        if (Get-Command sh -ErrorAction SilentlyContinue) {
            $shell = 'sh -o errexit -c'
        } elseif ($script:gitSh) {
            $shell = '"' + ($script:gitSh -replace '\\', '/') + '" -o errexit -c'
        } else {
            Set-ItResult -Skipped -Because 'no POSIX sh found'
            return
        }
        & $script:WriteShellConfig $shell "echo first line`nexit 3"

        $output = hk check --all 2>&1 | Out-String
        $LASTEXITCODE | Should -Not -Be 0 -Because "the script exits 3 on its second line, after printing; output:`n$output"
    }

    It 'accepts a quoted shell path that contains spaces' {
        if (-not $script:gitSh) { Set-ItResult -Skipped -Because 'Git for Windows sh not found'; return }
        & $script:WriteShellConfig ('"' + ($script:gitSh -replace '\\', '/') + '" -o errexit -c') 'echo quoted shell ok > ran.txt'

        $output = hk check --all 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because "hk check --all should succeed; output:`n$output"
        (Get-Content "ran.txt" -Raw).Trim() | Should -Be 'quoted shell ok'
    }

    It 'runs a POSIX script with substitutions and loops' {
        if (-not $script:gitSh) { Set-ItResult -Skipped -Because 'Git for Windows sh not found'; return }
        $posix = @'
out=$(printf 'a b')
for f in {{files}}; do
n=$((n + 1))
done
if [ "$n" -ge 1 ]; then printf '%s ok' "$out" > ran.txt; fi
'@
        & $script:WriteShellConfig ('"' + ($script:gitSh -replace '\\', '/') + '" -o errexit -c') $posix

        $output = hk check --all 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because "hk check --all should succeed; output:`n$output"
        (Get-Content "ran.txt" -Raw).Trim() | Should -Be 'a b ok'
    }
}
