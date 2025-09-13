BeforeAll {
    . $PSScriptRoot\Common.Tests.ps1
}

Describe "hk Basic Commands" {
    It "Should initialize hk configuration" {
        & $global:HkCommand init | Out-Null
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

        & $global:HkCommand validate | Out-Null
        $LASTEXITCODE | Should -Be 0
    }

    It "Should detect invalid configuration" {
        @'
amends "package://github.com/jdx/hk/releases/download/v1.2.0/hk@1.2.0#/Config.pkl"
invalid_syntax {
'@ | Out-File -FilePath "hk.pkl" -Encoding UTF8

        & $global:HkCommand validate 2>&1 | Out-Null
        $LASTEXITCODE | Should -Not -Be 0
    }
}