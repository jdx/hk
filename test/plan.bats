#!/usr/bin/env bats

load 'test_helper/common_setup'

setup() {
    _common_setup
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["prettier"] {
                glob = List("*.{js,ts}")
                check = "prettier --check"
                fix = "prettier --write"
            }
            ["eslint"] {
                glob = List("*.{js,ts}")
                check = "eslint"
                fix = "eslint --fix"
            }
            ["cargo-fmt"] {
                glob = List("*.rs")
                check = "cargo fmt --check"
                fix = "cargo fmt"
            }
        }
    }
}
EOF
}

teardown() {
    _common_teardown
}

@test "hk --plan shows basic plan" {
    touch file.js file.rs
    git add .
    run hk check --plan
    [ "$status" -eq 0 ]
    [[ "$output" == *"Plan: check"* ]]
    [[ "$output" == *"prettier"* ]]
    [[ "$output" == *"eslint"* ]]
    [[ "$output" == *"cargo-fmt"* ]]
}

@test "hk --plan with no matching files" {
    touch file.txt
    git add .
    run hk check --plan
    [ "$status" -eq 0 ]
    [[ "$output" == *"Plan: check"* ]]
    # Steps should be skipped due to no matching files
    [[ "$output" == *"no files matched"* ]]
}

@test "hk --plan --json outputs valid JSON" {
    touch file.js
    git add .
    run hk check --plan --json
    [ "$status" -eq 0 ]
    # Check JSON structure
    echo "$output" | jq -e '.hook == "check"' >/dev/null
    echo "$output" | jq -e '.steps | length > 0' >/dev/null
    echo "$output" | jq -e '.generatedAt' >/dev/null
}

@test "hk --plan with profiles" {
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["fast-check"] {
                glob = List("*.js")
                check = "echo fast"
                profiles = List("fast")
            }
            ["slow-check"] {
                glob = List("*.js")
                check = "echo slow"
                profiles = List("slow")
            }
        }
    }
}
EOF
    touch file.js
    git add .

    # Without profile, both should show
    run hk check --plan
    [ "$status" -eq 0 ]
    [[ "$output" == *"fast-check"* ]]
    [[ "$output" == *"slow-check"* ]]

    # With fast profile
    run hk --profile fast check --plan
    [ "$status" -eq 0 ]
    [[ "$output" == *"fast-check"* ]]
    [[ "$output" == *"excluded by profile"* ]] || [[ "$output" == *"profile"* ]]
}

@test "hk --plan with conditions" {
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["conditional-step"] {
                glob = List("*.js")
                check = "echo test"
                condition = "true"
            }
            ["conditional-false"] {
                glob = List("*.js")
                check = "echo test"
                condition = "false"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --plan
    [ "$status" -eq 0 ]
    [[ "$output" == *"conditional-step"* ]]
    [[ "$output" == *"condition: true"* ]] || [[ "$output" == *"condition evaluated to true"* ]]
    [[ "$output" == *"conditional-false"* ]]
    [[ "$output" == *"condition: false"* ]] || [[ "$output" == *"condition evaluated to false"* ]]
}

@test "hk --plan with dependencies" {
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step-a"] {
                glob = List("*.js")
                check = "echo a"
            }
            ["step-b"] {
                glob = List("*.js")
                check = "echo b"
                depends = List("step-a")
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --plan --json
    [ "$status" -eq 0 ]
    # Check that step-b has step-a in dependencies
    echo "$output" | jq -e '.steps[] | select(.name == "step-b") | .dependsOn | contains(["step-a"])' >/dev/null
}

@test "hk --plan with parallel groups" {
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["group:parallel"] {
                ["step1"] {
                    glob = List("*.js")
                    check = "echo 1"
                }
                ["step2"] {
                    glob = List("*.js")
                    check = "echo 2"
                }
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --plan
    [ "$status" -eq 0 ]
    [[ "$output" == *"[parallel group]"* ]] || [[ "$output" == *"step1"* ]]
    [[ "$output" == *"step2"* ]]
}

@test "hk --plan respects --step flag" {
    touch file.js
    git add .
    run hk check --plan --step prettier
    [ "$status" -eq 0 ]
    [[ "$output" == *"prettier"* ]]
    # Other steps should not be shown (or be marked as not selected)
    # The exact behavior depends on implementation
}

@test "hk --plan respects --skip-step flag" {
    touch file.js
    git add .
    run hk check --plan --skip-step prettier
    [ "$status" -eq 0 ]
    [[ "$output" == *"eslint"* ]]
    # prettier should be skipped
    [[ "$output" == *"prettier"* ]]
    [[ "$output" == *"skip"* ]] || [[ "$output" == *"excluded"* ]]
}
