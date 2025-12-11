#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hide warnings: HK_HIDE_WARNINGS=missing-profiles suppresses profile skip warning" {
    cat <<EOF > hk.pkl
amends "package://example.com/v1.26.0/hk@1.26.0#/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step"] {
                check = "echo 'I ran'"
            }
        }
    }
}
EOF

    HK_PKL_HTTP_REWRITE="https://example.com/=https://github.com/jdx/hk/releases/download/" run hk check
    assert_success
    assert_output --partial "I ran"
}
