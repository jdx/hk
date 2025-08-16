#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "dependent step proceeds when dependency's condition is false" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] {
                condition = "false"
                check = "echo 'A SHOULD NOT RUN'"
            }
            ["b"] {
                depends = List("a")
                check = "echo 'B RUNS'"
            }
        }
    }
}
EOF
    run hk check
    assert_success
    assert_output --partial "a – skipped: condition is false"
    assert_output --partial "B RUNS"
    refute_output --partial "A SHOULD NOT RUN"
}
