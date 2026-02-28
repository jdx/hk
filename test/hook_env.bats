#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "hook env: sets environment variables for steps" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        env {
            ["MY_HOOK_VAR"] = "hello"
        }
        steps {
            ["echo"] {
                check = "echo \$MY_HOOK_VAR"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "hello"
}

@test "hook env: step-level env takes precedence over hook-level env" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        env {
            ["MY_VAR"] = "from_hook"
        }
        steps {
            ["echo"] {
                env {
                    ["MY_VAR"] = "from_step"
                }
                check = "echo \$MY_VAR"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "from_step"
}

@test "hook env: does not leak to other hooks" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        env {
            ["HOOK_ONLY_VAR"] = "should_not_leak"
        }
        steps {
            ["echo"] { check = "echo pre-commit-ok" }
        }
    }
    ["check"] {
        steps {
            ["echo"] {
                check = "echo \${HOOK_ONLY_VAR:-not_set}"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "not_set"
}
