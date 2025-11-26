#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "git config: hk.jobs setting is respected" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo testing"
            }
        }
    }
}
EOF

    # Unset HK_JOBS so git config takes effect
    unset HK_JOBS
    git config --local hk.jobs 5

    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.jobs' | grep -q "5"
}

@test "git config: hk.failFast setting is respected" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo testing"
            }
        }
    }
}
EOF

    git config --local hk.failFast false

    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.fail_fast' | grep -q "false"
}

@test "git config: hk.profile setting adds profiles" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["slow-test"] {
                profiles = List("slow")
                check = "echo 'SLOW TEST'"
            }
        }
    }
}
EOF

    # Add multiple profiles via git config
    git config --local hk.profile slow
    git config --local --add hk.profile fast

    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.enabled_profiles[]' | grep -q "slow"
    echo "$output" | jq -r '.enabled_profiles[]' | grep -q "fast"
}

@test "git config: hk.exclude patterns are added to excludes" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo testing {{files}}"
                glob = List("**/*.txt")
            }
        }
    }
}
EOF

    mkdir -p excluded normal
    echo "test" > excluded/test.txt
    echo "test" > normal/test.txt
    git add .

    # Set exclude pattern via git config
    git config --local hk.exclude "excluded"

    run hk check --all
    [ "$status" -eq 0 ]
    echo "$output" | grep -q "normal/test.txt"
    ! echo "$output" | grep -q "excluded/test.txt"
}

@test "git config: multiple hk.exclude entries are combined" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
EOF

    # Add multiple exclude patterns
    git config --local hk.exclude "node_modules"
    git config --local --add hk.exclude "target"
    git config --local --add hk.exclude "*.min.js"

    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.exclude[]' | grep -q "node_modules"
    echo "$output" | jq -r '.exclude[]' | grep -q "target"
    echo "$output" | jq -r '.exclude[]' | grep -q "*.min.js"
}


@test "git config: hk.warnings adds warning tags" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
warnings = List("missing-profiles")
hooks {
    ["check"] {
        steps {
            ["test"] {
                profiles = List("nonexistent")
                check = "echo test"
            }
        }
    }
}
EOF

    git config --local hk.warnings "missing-profiles"

    run hk check
    [ "$status" -eq 0 ]
    echo "$output" | grep -q "missing profiles"
}

@test "git config: hk.hideWarnings suppresses warnings" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
warnings = List("missing-profiles")
hooks {
    ["check"] {
        steps {
            ["test"] {
                profiles = List("nonexistent")
                check = "echo test"
            }
        }
    }
}
EOF

    git config --local hk.hideWarnings "missing-profiles"

    run hk check
    [ "$status" -eq 0 ]
    ! echo "$output" | grep -q "missing profiles"
}

@test "git config: local config overrides global config" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
EOF

    # Unset environment variables so git config takes effect
    unset HK_JOBS

    # Set global config
    git config --global hk.jobs 10
    git config --global hk.failFast false

    # Override with local config
    git config --local hk.jobs 3
    git config --local hk.failFast true

    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.jobs' | grep -q "3"
    echo "$output" | jq -r '.fail_fast' | grep -q "true"
}

@test "git config: comma-separated values in single entry" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
EOF

    # Test comma-separated values in a single config entry
    git config --local hk.exclude "node_modules,target,dist"

    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.exclude[]' | grep -q "node_modules"
    echo "$output" | jq -r '.exclude[]' | grep -q "target"
    echo "$output" | jq -r '.exclude[]' | grep -q "dist"
}

@test "git config: environment variables override git config" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
EOF

    # Set git config
    git config --local hk.exclude "gitconfig-pattern"

    # Environment variable should be added (union semantics)
    export HK_EXCLUDE="env-pattern"

    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | jq -r '.exclude[]' | grep -q "gitconfig-pattern"
    echo "$output" | jq -r '.exclude[]' | grep -q "env-pattern"
}

@test "git config: CLI flags override git config" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "exit 1"
                glob = List("*.txt")
            }
            ["step2"] {
                check = "echo should run with --no-fail-fast"
                glob = List("*.txt")
            }
        }
    }
}
EOF

    echo "test" > test.txt
    git add .

    # Set fail-fast in git config
    git config --local hk.failFast true

    # CLI flag should override
    run hk check --no-fail-fast --all
    [ "$status" -ne 0 ]
    echo "$output" | grep -q "should run with --no-fail-fast"
}

@test "git config: hk.profile with negation" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["slow-test"] {
                profiles = List("slow")
                check = "echo 'SLOW TEST'"
            }
            ["fast-test"] {
                profiles = List("fast")
                check = "echo 'FAST TEST'"
            }
        }
    }
}
EOF

    # Enable profiles via git config
    git config --local hk.profile slow
    git config --local --add hk.profile fast
    git config --local --add hk.profile "!slow"

    run hk config dump
    [ "$status" -eq 0 ]
    # slow should be disabled
    ! echo "$output" | jq -r '.enabled_profiles[]' | grep -q "slow"
    # fast should still be enabled
    echo "$output" | jq -r '.enabled_profiles[]' | grep -q "fast"
}

@test "git config sources shows git config in precedence order" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
EOF

    # Set some git config values
    git config --global hk.jobs 10
    git config --local hk.jobs 5

    run hk config sources
    [ "$status" -eq 0 ]
    echo "$output" | grep -q "Git config (local"
    echo "$output" | grep -q "Git config (global"
    # Verify local appears before global in precedence
    local_line=$(echo "$output" | grep -n "Git config (local" | cut -d: -f1)
    global_line=$(echo "$output" | grep -n "Git config (global" | cut -d: -f1)
    [ "$local_line" -lt "$global_line" ]
}

