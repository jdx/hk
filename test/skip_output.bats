#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "skip output: disabled by profile" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["foo"] {
                profiles = List("needs-profile")
                check = "echo 'RUN'"
            }
        }
    }
}
EOF
    run hk check
    assert_success
    assert_output --partial "foo – skipped: disabled by profile"
    refute_output --partial "RUN"
}

@test "skip output: HK_SKIP_STEPS" {
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
    HK_SKIP_STEPS=foo run hk check
    assert_success
    assert_output --partial "foo – skipped: disabled via HK_SKIP_STEPS"
    refute_output --partial "RUN"
}

@test "skip output: condition false" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["foo"] {
                condition = "false"
                check = "echo 'RUN'"
            }
        }
    }
}
EOF
    run hk check
    assert_success
    assert_output --partial "foo – skipped: condition is false"
    refute_output --partial "RUN"
}
