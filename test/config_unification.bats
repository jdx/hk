#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export PKL_PATH="$PROJECT_ROOT/pkl"
}

teardown() {
    _common_teardown
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
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo testing {{files}}"
                glob = List("**/test.txt")
            }
        }
    }
}
EOF

    mkdir -p excluded normal
    echo "test" > excluded/test.txt
    echo "test" > normal/test.txt
    git add .

    export HK_EXCLUDE="excluded"
    run hk check --all
    [ "$status" -eq 0 ]
    echo "$output" | grep -q "normal/test.txt"
    ! echo "$output" | grep -q "excluded/test.txt"
}

@test "HK_EXCLUDE environment variable works with glob patterns" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo testing {{files}}"
                glob = List("**/*.js")
            }
        }
    }
}
EOF

    echo "test" > test.min.js
    echo "test" > test.js
    git add .

    export HK_EXCLUDE="**/*.min.js"
    run hk check --all
    [ "$status" -eq 0 ]
    echo "$output" | grep -q "test.js"
    ! echo "$output" | grep -q "test.min.js"
}

@test "--fail-fast flag works" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] {
                check = "exit 1"
                glob = List("*.txt")
            }
            ["step2"] {
                check = "echo should not run"
                glob = List("*.txt")
                depends = List("step1")
            }
        }
    }
}
EOF

    echo "test" > test.txt
    git add .

    run hk check --fail-fast --all
    [ "$status" -ne 0 ]
    ! echo "$output" | grep -q "should not run"
}

@test "--no-fail-fast flag works" {
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
                check = "echo should run"
                glob = List("*.txt")
            }
        }
    }
}
EOF

    echo "test" > test.txt
    git add .

    run hk check --no-fail-fast --all
    [ "$status" -ne 0 ]
    echo "$output" | grep -q "should run"
}

@test "--stash flag works" {
    cat > hk.pkl << EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo testing"
                glob = List("*.txt")
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
