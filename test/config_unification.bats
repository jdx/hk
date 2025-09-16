#!/usr/bin/env bats

setup() {
    export HK_TEMP_DIR="$(mktemp -d)"
    cd "$HK_TEMP_DIR"
    git init .
}

teardown() {
    cd ..
    rm -rf "$HK_TEMP_DIR"
}

@test "hk config dump shows effective settings" {
    run hk config dump
    [ "$status" -eq 0 ]
    echo "$output" | grep -q '"jobs"'
    echo "$output" | grep -q '"fail_fast"'
    echo "$output" | grep -q '"exclude"'
}

@test "hk config get retrieves specific values" {
    run hk config get fail_fast
    [ "$status" -eq 0 ]
}

@test "hk config sources shows configuration precedence" {
    run hk config sources
    [ "$status" -eq 0 ]
    echo "$output" | grep -q "CLI flags"
    echo "$output" | grep -q "Environment variables"
    echo "$output" | grep -q "Git config"
}

@test "HK_EXCLUDE environment variable works with paths" {
    cat > hk.pkl << 'EOF'
amends "Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["test"] {
                run = "echo testing"
            }
        }
    }
}
EOF

    mkdir -p excluded normal
    echo "test" > excluded/test.txt
    echo "test" > normal/test.txt

    export HK_EXCLUDE="excluded"
    run hk check --all
    [ "$status" -eq 0 ]
    # The excluded directory should not be processed
}

@test "HK_EXCLUDE environment variable works with glob patterns" {
    cat > hk.pkl << 'EOF'
amends "Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["test"] {
                run = "echo testing"
            }
        }
    }
}
EOF

    echo "test" > test.min.js
    echo "test" > test.js

    export HK_EXCLUDE="**/*.min.js"
    run hk check --all
    [ "$status" -eq 0 ]
    # The .min.js files should be excluded
}

@test "--fail-fast flag works" {
    cat > hk.pkl << 'EOF'
amends "Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["step1"] {
                run = "exit 1"
            }
            ["step2"] {
                run = "echo should not run"
            }
        }
    }
}
EOF

    echo "test" > test.txt

    run hk check --fail-fast --all
    [ "$status" -ne 0 ]
    ! echo "$output" | grep -q "should not run"
}

@test "--no-fail-fast flag works" {
    cat > hk.pkl << 'EOF'
amends "Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["step1"] {
                run = "exit 1"
            }
            ["step2"] {
                run = "echo should run"
            }
        }
    }
}
EOF

    echo "test" > test.txt

    run hk check --no-fail-fast --all
    [ "$status" -ne 0 ]
    echo "$output" | grep -q "should run"
}

@test "--stash flag works" {
    cat > hk.pkl << 'EOF'
amends "Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["test"] {
                run = "echo testing"
            }
        }
    }
}
EOF

    echo "test" > test.txt
    git add test.txt

    # Test stash=none
    run hk check --stash=none
    [ "$status" -eq 0 ]

    # Test stash=git
    run hk check --stash=git
    [ "$status" -eq 0 ]

    # Test stash=patch-file
    run hk check --stash=patch-file
    [ "$status" -eq 0 ]
}
