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
        $originalHkLog = $env:HK_LOG

        try {
            git init | Out-Null
            git config user.email "test@test.com"
            git config user.name "Test"

            $binDir = New-Item -ItemType Directory -Force -Path "node_modules/.bin"
            Set-Content -Path "node_modules/count.py" -Encoding ascii -Value @"
import os, sys
# One file per call: batches run concurrently, and appending to a shared file can fail.
with open('calls-%d.log' % os.getpid(), 'w') as f:
    f.write(''.join(os.path.basename(a) + '\n' for a in sys.argv[1:]))
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

            $env:HK_LOG = "debug"
            $output = hk check --all 2>&1 | Out-String
            $code = $LASTEXITCODE
            $code | Should -Be 0 -Because "hk check --all should succeed; output:`n$output"
            $logs = @(Get-ChildItem "calls-*.log")
            $logs.Count | Should -BeGreaterThan 1 -Because "the files should be split; output:`n$output"
            # Compare the names, not just the count: a repeated file plus an omitted one still sums to 300.
            $seen = @($logs | ForEach-Object { Get-Content $_.FullName } | Sort-Object)
            $expected = @(1..300 | ForEach-Object { "file_{0:D5}_abcdefghij.txt" -f $_ } | Sort-Object)
            ($seen -join ',') | Should -Be ($expected -join ',')
        } finally {
            $env:PATH = $originalPath
            $env:HK_LOG = $originalHkLog
            Set-Location $script:originalPath
            Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
