#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "check - using improved helpers" {
    # Simplified config setup
    setup_with_hook "check" '["a"] { check = "echo checking {{files}}" }'

    # Set up git state more easily
    setup_with_git_state committed hk.pkl
    echo "" >> hk.pkl
    echo "test" > test.js

    # Simplified assertion
    assert_hk_success check
    assert_output --partial "checking hk.pkl test.js"
}

@test "check files - using improved helpers" {
    setup_with_hook "check" '["a"] { check = "echo checking {{files}}" }'
    echo "test" > test.js

    assert_hk_success check test.js
    assert_output --partial "checking test.js"
}

@test "check files dir - using improved helpers" {
    setup_with_hook "check" '["a"] { check = "echo checking {{files}}" }'

    mkdir -p a/b/c
    echo "test" > a/b/c/test.js

    assert_hk_success check a
    assert_output --partial "checking a/b/c/test.js"
}

@test "check files w/ exclude dir - using improved helpers" {
    setup_with_config "amends \"$PKL_PATH/Config.pkl\"
hooks {
    ["check"] {
        steps {
            ["a"] {
                check = "echo checking {{files}}"
                exclude = ["excluded/**"]
            }
        }
    }
}'

    mkdir -p included excluded
    echo "test" > included/test.js
    echo "test" > excluded/test.js

    assert_hk_success check --all
    assert_output --partial "checking included/test.js"
    refute_output --partial "checking excluded/test.js"
}

@test "check specific file from subdirectory - using improved helpers" {
    setup_with_hook "check" '["a"] { check = "echo checking {{files}}" }'

    mkdir -p subdir
    cd subdir
    echo "test" > test.js

    assert_hk_success check test.js
    assert_output --partial "checking test.js"
}

@test "check with glob pattern - using improved helpers" {
    setup_with_config "amends \"$PKL_PATH/Config.pkl\"
hooks {
    ["check"] {
        steps {
            ["js_only"] {
                check = "echo checking {{files}}"
                glob = ["*.js"]
            }
        }
    }
}'

    create_test_files valid 2
    echo "not checked" > file.txt

    assert_hk_success check --all
    assert_output --partial "checking file1.js file2.js"
    refute_output --partial "file.txt"
}

@test "check with failing step - using improved helpers" {
    setup_with_failing_step "bad_check" "Check failed!"

    assert_hk_failure check --all
    assert_step_failed "bad_check"
    assert_output --partial "Check failed!"
}

@test "check with dependencies - using improved helpers" {
    setup_with_dependent_steps

    assert_hk_success check --all
    # Steps should execute in order
    assert_step_executed "step1"
    assert_step_executed "step2"
    assert_step_executed "step3"
}
