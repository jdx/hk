BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk Hook Installation" {
    BeforeEach {
        $script:TestDir = New-TestDirectory
    }

    AfterEach {
        Remove-TestDirectory $script:TestDir
    }
    It "Should install git hooks" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
hooks {
    ["pre-commit"] { steps {} }
    ["pre-push"] { steps {} }
}
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        & $global:HkCommand install | Out-Null
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

        & $global:HkCommand install | Out-Null
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

        & $global:HkCommand install | Out-Null
        ".git\hooks\pre-commit" | Should -Exist

        & $global:HkCommand uninstall | Out-Null
        $LASTEXITCODE | Should -Be 0
        ".git\hooks\pre-commit" | Should -Not -Exist
    }
}
