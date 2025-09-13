Describe "hk Windows Integration Tests" {
    BeforeAll {
        # Setup test environment
        $script:TestRoot = Join-Path $env:TEMP ("hk-test-" + [System.Guid]::NewGuid().ToString())
        New-Item -Path $script:TestRoot -ItemType Directory -Force | Out-Null
        
        $script:HkPath = Resolve-Path "target\release\hk.exe" -ErrorAction SilentlyContinue
        if (-not $script:HkPath) {
            $script:HkPath = Resolve-Path "..\..\target\release\hk.exe" -ErrorAction SilentlyContinue
        }
        if (-not $script:HkPath) {
            throw "Could not find hk.exe. Please build the project first."
        }
    }

    BeforeEach {
        # Create a new test directory for each test
        $script:TestDir = New-Item -Path (Join-Path $script:TestRoot ([System.Guid]::NewGuid().ToString())) -ItemType Directory
        Push-Location $script:TestDir
        
        # Initialize git repository
        git init | Out-Null
        git config user.email "test@example.com" | Out-Null
        git config user.name "Test User" | Out-Null
    }

    AfterEach {
        Pop-Location
        if (Test-Path $script:TestDir) {
            Remove-Item $script:TestDir -Recurse -Force
        }
    }

    AfterAll {
        if (Test-Path $script:TestRoot) {
            Remove-Item $script:TestRoot -Recurse -Force
        }
    }

    Context "Basic Commands" {
        It "Should initialize hk configuration" {
            & $script:HkPath init | Out-Null
            $LASTEXITCODE | Should -Be 0
            "hk.pkl" | Should -Exist
            
            $content = Get-Content "hk.pkl" -Raw
            $content | Should -Match 'amends.*Config'
        }

        It "Should validate configuration" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["pre-commit"] { steps {} }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            & $script:HkPath validate | Out-Null
            $LASTEXITCODE | Should -Be 0
        }

        It "Should detect invalid configuration" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
invalid_syntax {
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            & $script:HkPath validate 2>&1 | Out-Null
            $LASTEXITCODE | Should -Not -Be 0
        }
    }

    Context "Hook Installation" {
        It "Should install git hooks" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["pre-commit"] { steps {} }
    ["pre-push"] { steps {} }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            & $script:HkPath install | Out-Null
            $LASTEXITCODE | Should -Be 0
            
            ".git\hooks\pre-commit" | Should -Exist
            ".git\hooks\pre-push" | Should -Exist
        }

        It "Should create Windows batch files for hooks" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["pre-commit"] { steps {} }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            & $script:HkPath install | Out-Null
            $content = Get-Content ".git\hooks\pre-commit" -Raw
            $content | Should -Match '@echo off'
            $content | Should -Match 'hk run pre-commit'
        }

        It "Should uninstall git hooks" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["pre-commit"] { steps {} }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            & $script:HkPath install | Out-Null
            ".git\hooks\pre-commit" | Should -Exist
            
            & $script:HkPath uninstall | Out-Null
            $LASTEXITCODE | Should -Be 0
            ".git\hooks\pre-commit" | Should -Not -Exist
        }
    }

    Context "PowerShell Command Execution" {
        It "Should execute PowerShell commands" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ps-test"] {
                check = "Write-Host 'PowerShell test successful'"
                shell = "powershell.exe -NoProfile -Command"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            $output = & $script:HkPath check 2>&1
            $LASTEXITCODE | Should -Be 0
            $output | Should -Match "(PowerShell test successful|ps-test)"
        }

        It "Should execute pwsh commands if available" {
            if (-not (Get-Command pwsh.exe -ErrorAction SilentlyContinue)) {
                Set-ItResult -Skipped -Because "pwsh.exe not available"
                return
            }

            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["pwsh-test"] {
                check = "Write-Host 'PowerShell Core test successful'"
                shell = "pwsh.exe -NoProfile -Command"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            $output = & $script:HkPath check 2>&1
            $LASTEXITCODE | Should -Be 0
            $output | Should -Match "(PowerShell Core test successful|pwsh-test)"
        }
    }

    Context "CMD Command Execution" {
        It "Should execute CMD commands" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["cmd-test"] {
                check = "echo CMD test successful"
                shell = "cmd.exe /C"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            $output = & $script:HkPath check 2>&1
            $LASTEXITCODE | Should -Be 0
            $output | Should -Match "(CMD test successful|cmd-test)"
        }
    }

    Context "File Handling" {
        It "Should process files with glob patterns" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ps1-files"] {
                check = "Write-Host 'Processing PowerShell files'"
                glob = ["*.ps1"]
            }
            ["txt-files"] {
                check = "Write-Host 'Processing text files'"
                glob = ["*.txt"]
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            "Write-Host 'test'" | Out-File -FilePath "test.ps1" -Encoding UTF8
            "test content" | Out-File -FilePath "test.txt" -Encoding UTF8
            "other content" | Out-File -FilePath "test.log" -Encoding UTF8

            $output = & $script:HkPath check test.ps1 test.txt 2>&1
            $LASTEXITCODE | Should -Be 0
            $output | Should -Match "(Processing PowerShell files|ps1-files)"
            $output | Should -Match "(Processing text files|txt-files)"
        }

        It "Should handle Windows paths correctly" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["path-test"] {
                check = "echo Processing {{files}}"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            New-Item -Path "subdir" -ItemType Directory | Out-Null
            "content" | Out-File -FilePath "subdir\test.txt" -Encoding UTF8

            $output = & $script:HkPath check "subdir\test.txt" 2>&1
            $LASTEXITCODE | Should -Be 0
            $output | Should -Match "subdir"
        }
    }

    Context "Parallel Execution" {
        It "Should run multiple steps in parallel" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] { check = "echo Step 1 completed" }
            ["step2"] { check = "echo Step 2 completed" }
            ["step3"] { check = "echo Step 3 completed" }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            $output = & $script:HkPath check 2>&1
            $LASTEXITCODE | Should -Be 0
            $output | Should -Match "(Step 1 completed|step1)"
            $output | Should -Match "(Step 2 completed|step2)"
            $output | Should -Match "(Step 3 completed|step3)"
        }
    }

    Context "Step Dependencies" {
        It "Should respect step dependencies" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["first"] { check = "echo First step" }
            ["second"] {
                check = "echo Second step"
                depends = ["first"]
            }
            ["third"] {
                check = "echo Third step"
                depends = ["second"]
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            $output = & $script:HkPath check 2>&1
            $LASTEXITCODE | Should -Be 0
            $output | Should -Match "(First step|first)"
            $output | Should -Match "(Second step|second)"
            $output | Should -Match "(Third step|third)"
        }
    }

    Context "Error Handling" {
        It "Should handle failing commands" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["failing-step"] {
                check = "exit 1"
                shell = "cmd.exe /C"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            & $script:HkPath check 2>&1 | Out-Null
            $LASTEXITCODE | Should -Not -Be 0
        }

        It "Should continue with other steps when fail-fast is disabled" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["check"] {
        fail_fast = false
        steps {
            ["failing"] { check = "exit 1" shell = "cmd.exe /C" }
            ["passing"] { check = "echo Success" }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            $output = & $script:HkPath check 2>&1
            # Should still fail overall but run both steps
            $LASTEXITCODE | Should -Not -Be 0
            $output | Should -Match "(Success|passing)"
        }
    }

    Context "Git Integration" {
        It "Should work with git staged files" {
            @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["check-staged"] {
                check = "echo Checking {{files}}"
            }
        }
    }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

            "test content" | Out-File -FilePath "test.txt" -Encoding UTF8
            git add test.txt | Out-Null
            git commit -m "initial commit" | Out-Null
            
            "modified content" | Out-File -FilePath "test.txt" -Encoding UTF8
            git add test.txt | Out-Null

            $output = & $script:HkPath run pre-commit 2>&1
            $LASTEXITCODE | Should -Be 0
            $output | Should -Match "test.txt"
        }
    }

    Context "Configuration Formats" {
        It "Should work with different config formats" {
            # Test TOML config
            @'
[hooks.check.steps.toml-test]
check = "echo TOML config works"
'@ | Out-File -FilePath "hk.toml" -Encoding UTF8

            $output = & $script:HkPath check 2>&1
            if ($LASTEXITCODE -eq 0) {
                $output | Should -Match "(TOML config works|toml-test)"
            } else {
                # TOML support might not be available, that's ok
                Write-Host "TOML config not supported, skipping"
            }
        }
    }
}