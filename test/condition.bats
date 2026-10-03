#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "condition" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["a"] { fix = "echo ITWORKS > a.txt"; condition = "true" }
            ["b"] { fix = "echo ITWORKS > b.txt"; condition = "false" }
            ["c"] { fix = "echo ITWORKS > c.txt"; condition = "exec('echo ITWORKS') == 'ITWORKS\n'" }
        }
    }
}
EOF
    hk fix -v
    assert_file_exists a.txt
    assert_file_not_exists b.txt
    assert_file_exists c.txt
}

@test "step_condition evaluates once per step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["step"] {
                glob = "**/*"
                fix = ""
                workspace_indicator = "ws"
                step_condition = "true"
            }
        }
    }
}
EOF
    mkdir subdirA subdirB
    touch subdirA/ws subdirB/ws
    output=$(hk fix -v 2>&1)
    count=$(echo "$output" | grep -c "step: condition: true = true")
    assert_equal "$count" "1"
}

@test "exec_ok in a condition runs the step only when the command succeeds" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        steps {
            ["present"] { fix = "echo RAN > present.txt"; condition = "exec_ok('test -f marker.txt')" }
            ["absent"] { fix = "echo RAN > absent.txt"; condition = "exec_ok('test -f missing.txt')" }
            ["negated"] { fix = "echo RAN > negated.txt"; condition = "!exec_ok('test -f missing.txt')" }
            ["step-level"] { fix = "echo RAN > step-level.txt"; step_condition = "exec_ok('test -f missing.txt')" }
        }
    }
}
EOF
    touch marker.txt

    run hk fix
    assert_success
    assert_file_exists present.txt
    assert_file_not_exists absent.txt
    assert_file_exists negated.txt
    assert_file_not_exists step-level.txt
}

@test "a failing exec in a condition fails with a hint to use exec_ok" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        steps {
            ["gated"] { fix = "echo RAN > gated.txt"; condition = "exec('test -f missing.txt')" }
        }
    }
}
EOF

    run hk fix
    assert_failure
    assert_output --partial "exited with code 1"
    assert_output --partial "exec_ok()"
    assert_file_not_exists gated.txt
}

@test "exec with a missing or non-string argument is a configuration error, not a panic" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        steps {
            ["gated"] { fix = "echo RAN > gated.txt"; condition = "exec() == ''" }
        }
    }
}
EOF

    run hk fix
    assert_failure
    assert_output --partial "exec() expects exactly one string argument"
    refute_output --partial "panicked"
}

@test "exec with non-UTF-8 output is an error, not a panic" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        steps {
            ["gated"] { fix = "echo RAN > gated.txt"; condition = #"exec(\`printf '\\377'\`) == ''"# }
        }
    }
}
EOF

    run hk fix
    assert_failure
    assert_output --partial "not valid UTF-8"
    refute_output --partial "panicked"
}
