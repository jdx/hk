Describe 'unstaged deletion isolation' {
    BeforeAll {
        $script:deletionOriginalPath = Get-Location
        $script:deletionEnvironment = @{}
        foreach ($name in @('HK_LIBGIT2', 'HK_STASH_UNTRACKED', 'STASH_PROBE_FILE', 'STASH_PROBE_EXIT', 'GIT_CONFIG_GLOBAL', 'GIT_CONFIG_NOSYSTEM')) {
            $script:deletionEnvironment[$name] = [Environment]::GetEnvironmentVariable($name)
        }

        function Get-UnstagedDeletionSnapshot {
            [ordered]@{
                Index = git ls-files --stage | Out-String
                Staged = git diff --cached --binary | Out-String
                Worktree = git diff --binary | Out-String
                Status = git status --porcelain=v1 | Out-String
                Stashes = git for-each-ref refs/stash refs/hk/stash | Out-String
                Files = @(Get-ChildItem -File | Sort-Object Name | ForEach-Object {
                    @{ Name = $_.Name; Bytes = [Convert]::ToBase64String([IO.File]::ReadAllBytes($_.FullName)) }
                })
            } | ConvertTo-Json -Depth 4 -Compress
        }
    }

    BeforeEach {
        $testDir = Join-Path $TestDrive ([guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $testDir | Out-Null
        Set-Location $testDir
        $env:GIT_CONFIG_GLOBAL = Join-Path $testDir '.git/test-global-config'
        $env:GIT_CONFIG_NOSYSTEM = '1'
        git init -q --ref-format=files --template=
        git config user.email 'test@example.com'
        git config user.name 'Test'
        git config core.autocrlf false
        git config commit.gpgsign false
        $pklPath = (Resolve-Path $env:PKL_PATH).Path -replace '\\', '/'
        [IO.File]::WriteAllText("$testDir/hk.pkl", @"
amends "file:///$pklPath/Config.pkl"
hooks {
    ["pre-commit"] {
        stash = "git"
        stage = false
        fix = false
        steps {
            ["probe"] { check = "pwsh -NoProfile -File probe.ps1 {{files}}" }
        }
    }
}
"@)
        [IO.File]::WriteAllText("$testDir/probe.ps1", @'
$ErrorActionPreference = 'Stop'
if ([IO.File]::ReadAllText($env:STASH_PROBE_FILE) -cne "indexed`n") {
    throw 'indexed contents are not visible'
}
if ([IO.File]::ReadAllText('other.txt') -cne "staged`n") {
    throw 'staged edit is not visible'
}
if ([IO.File]::ReadAllText('extra.txt') -cne "baseline`n") {
    throw 'unstaged edit is visible'
}
if ($args.Count -ne 1 -or $args[0] -cne 'other.txt') {
    throw "unexpected file selection: $args"
}
Write-Output 'indexed contents visible'
exit [int]$env:STASH_PROBE_EXIT
'@)
        [IO.File]::WriteAllText("$testDir/tracked.txt", "indexed`n")
        [IO.File]::WriteAllText("$testDir/other.txt", "baseline`n")
        [IO.File]::WriteAllText("$testDir/extra.txt", "baseline`n")
        git add -A
        git commit -qm baseline
        $LASTEXITCODE | Should -Be 0
        $env:STASH_PROBE_FILE = 'tracked.txt'
        $env:STASH_PROBE_EXIT = '0'
    }

    AfterEach {
        Set-Location $script:deletionOriginalPath
        foreach ($name in $script:deletionEnvironment.Keys) {
            [Environment]::SetEnvironmentVariable($name, $script:deletionEnvironment[$name])
        }
    }

    It 'isolates <Scenario> with libgit2=<Backend>, untracked=<Untracked>, exit=<HookExit>' -ForEach @(
        foreach ($backend in @('0', '1')) {
            foreach ($untracked in @('0', '1')) {
                foreach ($scenario in @('deletion-only', 'deletion-and-edit', 'added-then-deleted')) {
                    foreach ($hookExit in @(0, 23)) {
                        @{ Backend = $backend; Untracked = $untracked; Scenario = $scenario; HookExit = $hookExit }
                    }
                }
            }
        }
    ) {
        $env:HK_LIBGIT2 = $Backend
        $env:HK_STASH_UNTRACKED = $Untracked
        $env:STASH_PROBE_EXIT = $HookExit
        [IO.File]::WriteAllText("$testDir/other.txt", "staged`n")
        git add other.txt
        if ($Scenario -eq 'added-then-deleted') {
            $env:STASH_PROBE_FILE = 'added.txt'
            [IO.File]::WriteAllText("$testDir/added.txt", "indexed`n")
            git add added.txt
        }
        Remove-Item -LiteralPath $env:STASH_PROBE_FILE
        if ($Scenario -eq 'deletion-and-edit') {
            [IO.File]::WriteAllText("$testDir/extra.txt", "unstaged`n")
        }
        $before = Get-UnstagedDeletionSnapshot

        $output = hk run pre-commit --check -v 2>&1 | Out-String
        $hookStatus = $LASTEXITCODE
        $output | Should -Match 'indexed contents visible'
        if ($HookExit -eq 0) {
            $hookStatus | Should -Be 0 -Because $output
        } else {
            $hookStatus | Should -Not -Be 0 -Because $output
        }
        Test-Path -LiteralPath $env:STASH_PROBE_FILE | Should -BeFalse
        (Get-UnstagedDeletionSnapshot) | Should -BeExactly $before
    }

    It 'leaves unstaged deletions visible with stash=none and libgit2=<Backend>' -ForEach @(
        @{ Backend = '0' }
        @{ Backend = '1' }
    ) {
        $env:HK_LIBGIT2 = $Backend
        [IO.File]::WriteAllText("$testDir/probe.ps1", @'
if (Test-Path tracked.txt) { throw 'unstaged deletion was hidden' }
if ([IO.File]::ReadAllText('extra.txt') -cne "unstaged`n") { throw 'unstaged edit was hidden' }
Write-Output 'unstaged changes visible'
'@)
        [IO.File]::WriteAllText("$testDir/other.txt", "staged`n")
        git add probe.ps1 other.txt
        Remove-Item -LiteralPath tracked.txt
        [IO.File]::WriteAllText("$testDir/extra.txt", "unstaged`n")
        $before = Get-UnstagedDeletionSnapshot

        $output = hk run pre-commit --check --stash none -v 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because $output
        $output | Should -Match 'unstaged changes visible'
        (Get-UnstagedDeletionSnapshot) | Should -BeExactly $before
    }

    It 'leaves sparse-checkout entries absent with libgit2=<Backend>' -ForEach @(
        @{ Backend = '0' }
        @{ Backend = '1' }
    ) {
        $env:HK_LIBGIT2 = $Backend
        New-Item -ItemType Directory -Path included, excluded | Out-Null
        [IO.File]::WriteAllText("$testDir/excluded/input.txt", "excluded`n")
        git add excluded
        git commit -qm sparse
        git sparse-checkout set --cone included
        $LASTEXITCODE | Should -Be 0
        [IO.File]::WriteAllText("$testDir/other.txt", "staged`n")
        git add other.txt
        $before = Get-UnstagedDeletionSnapshot

        $output = hk run pre-commit --check -v 2>&1 | Out-String
        $LASTEXITCODE | Should -Be 0 -Because $output
        $output | Should -Match 'No unstaged changes to stash'
        Test-Path excluded/input.txt | Should -BeFalse
        (Get-UnstagedDeletionSnapshot) | Should -BeExactly $before
    }
}
