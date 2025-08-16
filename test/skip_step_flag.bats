#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "--skip-step skips named step with message" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["foo"] {
                check = "echo 'RUN'"
            }
        }
    }
}
EOF
    run hk check --skip-step foo
    assert_success
    assert_output --partial "foo – skipped: disabled via --skip-step foo"
    refute_output --partial "RUN"
}
