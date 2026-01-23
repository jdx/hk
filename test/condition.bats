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

@test "condition evaluates once per step" {
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
                condition = "true"
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
