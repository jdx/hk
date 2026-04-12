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

    It '{{files}} template passes clean arguments on Windows (no literal quotes)' {
        # Regression test for https://github.com/jdx/hk/discussions/823
        # Rust's `Command::arg` on Windows applies MSVCRT-style argv escaping
        # which conflicts with cmd.exe's own quoting, mangling the already
        # shell-quoted `{{files}}` string into args with literal `"` chars.
        $testDir = Join-Path $TestDrive ([System.Guid]::NewGuid().ToString())
        New-Item -ItemType Directory -Path $testDir | Out-Null
        Set-Location $testDir

        try {
            git init | Out-Null
            git config user.email "test@test.com"
            git config user.name "Test"

            # A helper script that fails if any argument contains a literal
            # double-quote character, and also fails if any claimed file path
            # does not actually exist on disk.
            $checker = @"
import sys
bad = [a for a in sys.argv[1:] if '"' in a]
if bad:
    print('LITERAL QUOTES IN ARGS:', bad)
    sys.exit(2)
import os
missing = [a for a in sys.argv[1:] if not os.path.exists(a)]
if missing:
    print('MISSING FILES:', missing)
    sys.exit(3)
print('OK', sys.argv[1:])
sys.exit(0)
"@
            $checker | Out-File -FilePath "check_args.py" -Encoding utf8

            $config = @"
amends "$env:PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["files-template"] {
                glob = "*.txt"
                check = "python check_args.py {{files}}"
            }
        }
    }
}
"@
            $config | Out-File -FilePath "hk.pkl" -Encoding utf8

            "simple" | Out-File -FilePath "simple.txt" -Encoding utf8
            "spaced" | Out-File -FilePath "hello world.txt" -Encoding utf8

            git add -A
            git commit -m "initial" | Out-Null

            $output = hk check --all 2>&1 | Out-String
            $LASTEXITCODE | Should -Be 0
            $output | Should -Not -Match 'LITERAL QUOTES IN ARGS'
            $output | Should -Not -Match 'MISSING FILES'
            # Positive assertion: if `{{files}}` silently expanded to nothing,
            # check_args.py would print `OK []` and the negative matches above
            # would still pass. Require both files to show up in the OK line.
            $output | Should -Match 'OK .*simple\.txt'
            $output | Should -Match 'OK .*hello world\.txt'
        } finally {
            Set-Location $script:originalPath
            Remove-Item -Path $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}
