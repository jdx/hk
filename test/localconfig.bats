#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "loads hk.local.pkl from project directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["echo"] { check = "env" }
        }
    }
}
EOF

    cat <<EOF > hk.local.pkl
amends "./hk.pkl"
import "./hk.pkl" as repo_config

hooks = (repo_config.hooks) {
    ["check"] {
        steps {
            ["echo"] {
                env {
                    ["STEP_VAR"] = "step_value"
                }
            }
            ["new step"] {
                check = "echo 'hello'"
            }
        }
    }
}
EOF

    run hk run check
    assert_success
    assert_output --partial "STEP_VAR=step_value"
    assert_output --partial "hello"
}
