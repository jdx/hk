Describe 'version' {
    It 'displays version information' {
        $output = hk --version
        $output | Should -Match '^hk \d+\.\d+\.\d+'
    }

    It 'displays version with -V flag' {
        $output = hk -V
        $output | Should -Match '^hk \d+\.\d+\.\d+'
    }
}
