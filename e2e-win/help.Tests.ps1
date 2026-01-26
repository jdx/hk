Describe 'help' {
    It 'displays help information' {
        $output = hk --help
        $output | Should -Match 'Usage:'
        $output | Should -Match 'Commands:'
    }

    It 'displays help with -h flag' {
        $output = hk -h
        $output | Should -Match 'Usage:'
    }

    It 'shows check subcommand help' {
        $output = hk check --help
        $output | Should -Match 'check'
    }

    It 'shows fix subcommand help' {
        $output = hk fix --help
        $output | Should -Match 'fix'
    }

    It 'shows init subcommand help' {
        $output = hk init --help
        $output | Should -Match 'init'
    }

    It 'shows install subcommand help' {
        $output = hk install --help
        $output | Should -Match 'install'
    }
}
