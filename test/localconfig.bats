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

    run hk run check
    assert_success
    refute_output --partial "STEP_VAR=step_value"
    refute_output --partial "hello"

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

@test "hk.local.pkl takes precedence over hk.pkl" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["greeting"] { check = "echo 'from hk.pkl'" }
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
            ["greeting"] { check = "echo 'from hk.local.pkl'" }
        }
    }
}
EOF

    run hk run check
    assert_success
    assert_output --partial "from hk.local.pkl"
    refute_output --partial "from hk.pkl"
}
