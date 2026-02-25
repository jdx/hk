Describe 'help' {
    It 'displays help information' {
        $output = hk --help 2>&1 | Out-String
        $output | Should -Match 'Usage:'
        $output | Should -Match 'Commands:'
    }

    It 'displays help with -h flag' {
        $output = hk -h 2>&1 | Out-String
        $output | Should -Match 'Usage:'
    }

    It 'shows check subcommand help' {
        $output = hk check --help 2>&1 | Out-String
        $output | Should -Match 'check'
    }

    It 'shows fix subcommand help' {
        $output = hk fix --help 2>&1 | Out-String
        $output | Should -Match 'fix'
    }

    It 'shows init subcommand help' {
        $output = hk init --help 2>&1 | Out-String
        $output | Should -Match 'init'
    }

    It 'shows install subcommand help' {
        $output = hk install --help 2>&1 | Out-String
        $output | Should -Match 'install'
    }
}
