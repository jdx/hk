Describe 'batching' {
    BeforeAll {
        $script:originalPath = Get-Location
    }

    AfterAll {
        Set-Location $script:originalPath
    }

    It 'structured argv batches cmd shims below the cmd.exe line limit' {
        # cmd.exe rejects a command line over 8191 characters ("The command line
        # is too long."). A shim's caret-escaped files are 4 characters longer
        # each (8 under node_modules\.bin), so 300 files fit CreateProcess's
        # limit but not cmd.exe's and must be split.
        $testDir = Join-Path $TestDrive ([System.Guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $testDir | Out-Null
        Set-Location $testDir
        $originalPath = $env:PATH

        try {
            git init | Out-Null
            git config user.email "test@test.com"
            git config user.name "Test"

            $binDir = New-Item -ItemType Directory -Force -Path "node_modules/.bin"
            Set-Content -Path "node_modules/count.py" -Encoding ascii -Value @"
import sys
with open('calls.log', 'a') as f:
    f.write(str(len(sys.argv) - 1) + '\n')
"@
            Set-Content -Path "$binDir/argv-count.cmd" -Encoding ascii -Value @'
@ECHO off
python "%~dp0\..\count.py" %*
'@
            New-Item -ItemType Directory -Path src | Out-Null
            1..300 | ForEach-Object {
                Set-Content -Path ("src/file_{0:D5}_abcdefghij.txt" -f $_) -Value "x"
            }
            git add -A
            git commit -m "initial" | Out-Null
            $env:PATH = "node_modules/.bin;$originalPath"

            $pklPath = (Resolve-Path $env:PKL_PATH).Path -replace '\\', '/'
            $pklUri = "file:///$pklPath/Config.pkl"
            $config = @"
amends "$pklUri"

hooks {
    ["check"] {
        steps {
            ["count"] {
                glob = "src/*.txt"
                check = new Command {
                    argv = List("argv-count", "{{files}}")
                }
            }
        }
    }
}
"@
            Set-Content -Path "hk.pkl" -Value $config -Encoding ascii

            $output = hk check --all 2>&1 | Out-String
            $LASTEXITCODE | Should -Be 0 -Because "hk check --all should succeed; output:`n$output"
            $calls = @(Get-Content "calls.log" | ForEach-Object { [int]$_ })
            $calls.Count | Should -BeGreaterThan 1
            ($calls | Measure-Object -Sum).Sum | Should -Be 300
        } finally {
            $env:PATH = $originalPath
            Set-Location $script:originalPath
            Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
