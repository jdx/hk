#!/usr/bin/env bats

load 'test_helper/common_setup'

setup() {
    _common_setup
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["multi-reason"] {
                glob = List("*.js")
                check = "echo test"
                profiles = List("test")
                condition = "true"
            }
            ["simple"] {
                glob = List("*.rs")
                check = "echo simple"
            }
        }
    }
}
EOF
}

teardown() {
    _common_teardown
}

@test "hk --plan --why shows detailed reasons for all steps" {
    touch file.js file.rs
    git add .
    run hk check --plan --why
    [ "$status" -eq 0 ]
    # Should show detailed reasons
    [[ "$output" == *"multi-reason"* ]]
    [[ "$output" == *"simple"* ]]
    # Should show multiple reasons for multi-reason step if applicable
}

@test "hk --plan --why <step> shows reasons for specific step" {
    touch file.js
    git add .
    run hk check --plan --why multi-reason
    [ "$status" -eq 0 ]
    [[ "$output" == *"multi-reason"* ]]
    # Should not show other steps or show them minimally
    # The simple step should not have detailed output
}

@test "hk --plan --why with no matching files shows skip reason" {
    touch file.txt
    git add .
    run hk check --plan --why
    [ "$status" -eq 0 ]
    [[ "$output" == *"no files matched"* ]]
}

@test "hk --plan --why shows profile exclusion reasons" {
    touch file.js
    git add .
    # Run without the test profile
    run hk check --plan --why multi-reason
    [ "$status" -eq 0 ]
    [[ "$output" == *"multi-reason"* ]]
    # Should show profile-related reason
    [[ "$output" == *"profile"* ]] || [[ "$output" == *"not enabled"* ]]
}

@test "hk --plan --why shows condition evaluation" {
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["cond-true"] {
                glob = List("*.js")
                check = "echo test"
                condition = "true"
            }
            ["cond-false"] {
                glob = List("*.js")
                check = "echo test"
                condition = "false"
            }
            ["cond-complex"] {
                glob = List("*.js")
                check = "echo test"
                condition = "1 + 1 == 2"
            }
        }
    }
}
EOF
    touch file.js
    git add .
    run hk check --plan --why
    [ "$status" -eq 0 ]
    # Should show condition evaluations
    [[ "$output" == *"condition"* ]]
    [[ "$output" == *"true"* ]]
    [[ "$output" == *"false"* ]]
}

@test "hk --plan --why shows CLI selection reasons" {
    touch file.js file.rs
    git add .
    run hk check --plan --why --step multi-reason
    [ "$status" -eq 0 ]
    [[ "$output" == *"multi-reason"* ]]
    # Should indicate it was selected via CLI
    [[ "$output" == *"--step"* ]] || [[ "$output" == *"included via"* ]]
}

@test "hk --plan --why shows skip-step reasons" {
    touch file.js
    git add .
    run hk check --plan --why --skip-step multi-reason
    [ "$status" -eq 0 ]
    [[ "$output" == *"multi-reason"* ]]
    # Should indicate it was skipped via CLI
    [[ "$output" == *"--skip-step"* ]] || [[ "$output" == *"excluded"* ]] || [[ "$output" == *"disabled"* ]]
}
