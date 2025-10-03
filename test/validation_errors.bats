#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "validation fails when step has stage but no fix" {
    # Create config with step that has stage but no fix
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["test-step"] {
                glob = "*.txt"
                check = "echo checking"
                stage = "*"
            }
        }
    }
}
EOF

    # Any command that loads config should fail
    run hk check
    assert_failure
    assert_output --partial "Step 'test-step' in hook 'pre-commit' has 'stage' attribute but no 'fix' command"
    assert_output --partial "Steps that stage files must have a fix command"
}

@test "validation passes when step has stage and fix" {
    # Create config with step that has both stage and fix
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["test-step"] {
                glob = "*.txt"
                check = "echo checking"
                fix = "echo fixing"
                stage = "*"
            }
        }
    }
}
EOF

    # Should not fail
    run hk validate
    assert_success
}

@test "validation passes when step has fix but no stage" {
    # Create config with step that has fix but no stage
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["test-step"] {
                glob = "*.txt"
                check = "echo checking"
                fix = "echo fixing"
            }
        }
    }
}
EOF

    # Should not fail
    run hk validate
    assert_success
}

@test "validation passes when step has neither stage nor fix" {
    # Create config with step that has neither stage nor fix (check only)
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["test-step"] {
                glob = "*.txt"
                check = "echo checking"
            }
        }
    }
}
EOF

    # Should not fail
    run hk validate
    assert_success
}

@test "validation fails when step in group has stage but no fix" {
    # Create config with step in a group that has stage but no fix
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["test-group"] = new Group {
                steps {
                    ["test-step"] {
                        glob = "*.txt"
                        check = "echo checking"
                        stage = "*"
                    }
                }
            }
        }
    }
}
EOF

    # Any command that loads config should fail
    run hk check
    assert_failure
    assert_output --partial "Step 'test-step' in group 'test-group' of hook 'pre-commit' has 'stage' attribute but no 'fix' command"
    assert_output --partial "Steps that stage files must have a fix command"
}

@test "validation validates all hooks" {
    # Create config with invalid step in a different hook
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
    ["pre-push"] {
        steps {
            ["test-step"] {
                glob = "*.txt"
                check = "echo checking"
                stage = "*"
            }
        }
    }
}
EOF

    # Should fail for pre-push hook
    run hk check
    assert_failure
    assert_output --partial "Step 'test-step' in hook 'pre-push' has 'stage' attribute but no 'fix' command"
}
