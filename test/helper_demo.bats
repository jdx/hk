#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "helper demo: assert_hk_success simplifies success checks" {
    setup_with_config 'amends "'"$PKL_PATH"'/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["echo"] {
        shell = "echo hello"
        glob = List("*")
      }
    }
  }
}'
    # Create a file so the step has something to process
    echo "test" > test.txt

    assert_hk_success check --all
    assert_line --partial "hello"
}

@test "helper demo: assert_file_modified detects file changes" {
    echo "original" > test.txt
    local original_mtime=$(get_file_mtime test.txt)

    sleep 1  # Ensure mtime will be different
    echo "modified" > test.txt

    assert_file_modified test.txt "$original_mtime"
}

@test "helper demo: setup_with_builtin simplifies builtin testing" {
    # Create a JavaScript file with issues
    echo "console.log('test')" > test.js  # Missing semicolon

    # Set up with prettier builtin
    setup_with_builtin prettier

    # Run check (will fail due to formatting issues)
    run hk check --all
    assert_failure
    assert_line --partial "prettier"
}

@test "helper demo: setup_with_git_state creates specific git states" {
    # Set up with staged files
    setup_with_git_state staged file1.txt file2.txt

    # Verify files are staged
    run git status --short
    assert_output --partial "A  file1.txt"
    assert_output --partial "A  file2.txt"

    # Test that git is not clean
    assert_git_dirty
}

@test "helper demo: setup_project_with_files creates typed projects" {
    setup_project_with_files javascript 3

    # Verify files were created
    assert_file_exists src/file1.js
    assert_file_exists src/file2.js
    assert_file_exists src/file3.js

    # Verify content
    assert_file_contains src/file1.js "console.log"
}

@test "helper demo: assert_step_skipped checks skip reasons" {
    setup_with_profile "production" false

    # Run without the profile active
    assert_hk_success check --all
    assert_step_skipped "test_step" "profile-not-enabled"
}

@test "helper demo: setup_with_failing_step for error testing" {
    setup_with_failing_step "bad_step" "Something went wrong"

    assert_hk_failure check --all
    assert_step_failed "bad_step"
    assert_output --partial "Something went wrong"
}

@test "helper demo: complex scenario with multiple helpers" {
    # Set up environment
    setup_with_env HK_FAIL_FAST=true HK_JSON=false

    # Create a project with dependent steps
    setup_with_dependent_steps

    # Create some test files
    create_test_files valid 2

    # Track timing
    setup_with_timing "$TEST_TEMP_DIR/test_timing.json"

    # Run and verify
    assert_hk_success check --all
    assert_line --partial "step1"
    assert_line --partial "step2"
    assert_line --partial "step3"

    # Verify timing file was created
    assert_file_exists "$TEST_TEMP_DIR/test_timing.json"
}

@test "helper demo: assert_output_matches with regex" {
    setup_with_config 'amends "'"$PKL_PATH"'/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["timestamp"] {
        shell = "date +%Y-%m-%d"
      }
    }
  }
}'

    assert_hk_success check --all
    # Check output matches date format YYYY-MM-DD
    assert_output_matches "[0-9]{4}-[0-9]{2}-[0-9]{2}"
}

@test "helper demo: setup_complex_directory_structure" {
    setup_complex_directory_structure

    # Verify structure was created
    assert_file_exists src/components/App.js
    assert_file_exists src/utils/helper.js
    assert_file_exists src/tests/App.test.js
    assert_file_exists docs/README.md
    assert_file_exists config/settings.json
    assert_file_exists scripts/build.sh
    assert_file_exists .gitignore
    assert_file_exists .prettierrc

    # Verify script is executable
    assert_file_permissions scripts/build.sh 755
}
