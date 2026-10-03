Describe 'cancel' {
    BeforeAll {
        $script:originalPath = Get-Location
    }

    AfterAll {
        Set-Location $script:originalPath
    }

    It 'fail-fast cancellation ends the processes behind cmd.exe' {
        # A step runs as `cmd.exe /c <tool>`. Cancelling it terminated cmd.exe
        # only, so the tool kept writing after hk had released its locks.
        $testDir = Join-Path $TestDrive ([System.Guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $testDir | Out-Null
        Set-Location $testDir

        try {
            git init | Out-Null
            git config user.email "test@test.com"
            git config user.name "Test"

            Set-Content -Path "hk_cancel_writer.py" -Encoding ascii -Value @"
import time
end = time.time() + 60
while time.time() < end:
    with open('writer.log', 'a') as f:
        f.write('x\n')
    time.sleep(0.2)
"@
            "test" | Out-File -FilePath "test.txt" -Encoding ascii
            git add -A
            git commit -m "initial" | Out-Null

            $pklPath = (Resolve-Path $env:PKL_PATH).Path -replace '\\', '/'
            $pklUri = "file:///$pklPath/Config.pkl"
            $config = @"
amends "$pklUri"

hooks {
    ["check"] {
        steps {
            ["slow"] {
                check = "python hk_cancel_writer.py"
            }
            ["fails"] {
                check = "ping -n 3 127.0.0.1 >nul & exit 1"
            }
        }
    }
}
"@
            Set-Content -Path "hk.pkl" -Value $config -Encoding ascii

            $watch = [System.Diagnostics.Stopwatch]::StartNew()
            cmd /c "hk check --all > hk-output.txt 2>&1"
            $watch.Stop()
            $LASTEXITCODE | Should -Not -Be 0
            # A tool left running also holds hk's output pipe open, so hk is
            # slow to exit as well.
            $watch.Elapsed.TotalSeconds | Should -BeLessThan 30
            $writer = Get-Content "writer.log" -ErrorAction SilentlyContinue
            $writer | Should -Not -BeNullOrEmpty -Because "the slow step should have started"
            $before = @($writer).Count
            Start-Sleep -Seconds 2
            @(Get-Content "writer.log").Count | Should -Be $before -Because "the cancelled step's tool should have been ended"
            @(Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like '*hk_cancel_writer.py*' }).Count |
                Should -Be 0 -Because "no process behind the cancelled step should survive"
        } finally {
            Get-CimInstance Win32_Process |
                Where-Object { $_.CommandLine -like '*hk_cancel_writer.py*' } |
                ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
            Set-Location $script:originalPath
            Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
