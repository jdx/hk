#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "dependent step proceeds when dependency has no command for run type" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
display_skip_reasons = List("no-command-for-run-type")
hooks {
    ["check"] {
        steps {
            ["only_fix"] {
                // Has only a fix command, so for `hk check` there is no run command
                fix = "echo 'WILL_NOT_RUN'"
            }
            ["needs_dep"] {
                depends = List("only_fix")
                check = "echo 'RUN'"
            }
        }
    }
}
EOF
    run hk check
    assert_success
    assert_output --partial "only_fix – skipped: no command for run type"
    assert_output --partial "RUN"
    refute_output --partial "WILL_NOT_RUN"
}
