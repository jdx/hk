#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "git config: hk.skipSteps configuration" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "echo 'STEP1 RAN'"
            }
            ["step2"] {
                check = "echo 'STEP2 RAN'"
            }
            ["step3"] {
                check = "echo 'STEP3 RAN'"
            }
        }
    }
}
EOF

    # Configure git to skip step2
    git config --local hk.skipSteps "step2"

    run hk check --all
    [ "$status" -eq 0 ]
    echo "$output" | grep -q "STEP1 RAN"
    ! echo "$output" | grep -q "STEP2 RAN"
    echo "$output" | grep -q "STEP3 RAN"
}

@test "git config: multiple hk.skipSteps entries" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "echo 'STEP1 RAN'"
            }
            ["step2"] {
                check = "echo 'STEP2 RAN'"
            }
            ["step3"] {
                check = "echo 'STEP3 RAN'"
            }
        }
    }
}
EOF

    # Skip multiple steps
    git config --local hk.skipSteps "step1"
    git config --local --add hk.skipSteps "step3"

    run hk check --all
    [ "$status" -eq 0 ]
    ! echo "$output" | grep -q "STEP1 RAN"
    echo "$output" | grep -q "STEP2 RAN"
    ! echo "$output" | grep -q "STEP3 RAN"
}

@test "git config: hk.skipHook configuration" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo 'CHECK HOOK RAN'"
            }
        }
    }
    ["pre-commit"] {
        steps {
            ["test"] {
                check = "echo 'PRE-COMMIT HOOK RAN'"
            }
        }
    }
}
EOF

    # Skip the check hook
    git config --local hk.skipHook "check"

    run hk check --all
    [ "$status" -eq 0 ]
    ! echo "$output" | grep -q "CHECK HOOK RAN"
}

@test "environment variable HK_SKIP_STEPS still works" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "echo 'STEP1 RAN'"
            }
            ["step2"] {
                check = "echo 'STEP2 RAN'"
            }
        }
    }
}
EOF

    export HK_SKIP_STEPS="step1"

    run hk check --all
    [ "$status" -eq 0 ]
    ! echo "$output" | grep -q "STEP1 RAN"
    echo "$output" | grep -q "STEP2 RAN"
}

@test "union semantics: git config and env vars combine" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "echo 'STEP1 RAN'"
            }
            ["step2"] {
                check = "echo 'STEP2 RAN'"
            }
            ["step3"] {
                check = "echo 'STEP3 RAN'"
            }
        }
    }
}
EOF

    # Skip step1 via git config
    git config --local hk.skipSteps "step1"

    # Skip step2 via environment variable
    export HK_SKIP_STEPS="step2"

    run hk check --all
    [ "$status" -eq 0 ]
    # Both step1 and step2 should be skipped (union semantics)
    ! echo "$output" | grep -q "STEP1 RAN"
    ! echo "$output" | grep -q "STEP2 RAN"
    echo "$output" | grep -q "STEP3 RAN"
}

@test "CLI flag --skip-step overrides configuration" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "echo 'STEP1 RAN'"
            }
            ["step2"] {
                check = "echo 'STEP2 RAN'"
            }
            ["step3"] {
                check = "echo 'STEP3 RAN'"
            }
        }
    }
}
EOF

    # Skip step1 via git config
    git config --local hk.skipSteps "step1"

    # Additionally skip step3 via CLI
    run hk check --all --skip-step step3
    [ "$status" -eq 0 ]
    ! echo "$output" | grep -q "STEP1 RAN"
    echo "$output" | grep -q "STEP2 RAN"
    ! echo "$output" | grep -q "STEP3 RAN"
}

@test "user config (.hkrc.pkl) skip configuration" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "echo 'STEP1 RAN'"
            }
            ["step2"] {
                check = "echo 'STEP2 RAN'"
            }
        }
    }
}
EOF

    cat > ~/.hkrc.pkl << EOF
amends "$PKL_PATH/UserConfig.pkl"
defaults {
    skip_steps = List("step1")
}
EOF

    run hk check --all
    [ "$status" -eq 0 ]
    ! echo "$output" | grep -q "STEP1 RAN"
    echo "$output" | grep -q "STEP2 RAN"
}

@test "config dump includes skip configuration" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
EOF

    git config --local hk.skipSteps "test-step"
    git config --local hk.skipHook "test-hook"

    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.skip_steps[]' | grep -q "test-step"
    echo "$output" | jq -r '.skip_hooks[]' | grep -q "test-hook"
}

@test "config get skip_steps works" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
EOF

    git config --local hk.skipSteps "step1"
    git config --local --add hk.skipSteps "step2"

    run hk config get skip_steps
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.[]' | grep -q "step1"
    echo "$output" | jq -r '.[]' | grep -q "step2"
}

@test "config get skip_hooks works" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
EOF

    git config --local hk.skipHook "pre-commit"
    git config --local --add hk.skipHook "pre-push"

    run hk config get skip_hooks
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.[]' | grep -q "pre-commit"
    echo "$output" | jq -r '.[]' | grep -q "pre-push"
}

@test "backward compatibility: hk.skipStep singular form" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo 'TEST RAN'"
            }
        }
    }
}
EOF

    # Use singular form
    git config --local hk.skipStep "test"

    run hk check --all
    [ "$status" -eq 0 ]
    ! echo "$output" | grep -q "TEST RAN"
}

@test "comma-separated skip values in git config" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "echo 'STEP1 RAN'"
            }
            ["step2"] {
                check = "echo 'STEP2 RAN'"
            }
            ["step3"] {
                check = "echo 'STEP3 RAN'"
            }
        }
    }
}
EOF

    # Use comma-separated values
    git config --local hk.skipSteps "step1,step3"

    run hk check --all
    [ "$status" -eq 0 ]
    ! echo "$output" | grep -q "STEP1 RAN"
    echo "$output" | grep -q "STEP2 RAN"
    ! echo "$output" | grep -q "STEP3 RAN"
}
