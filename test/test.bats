#!/usr/bin/env bats

setup() {
    load 'test_helper/bats-support/load'
    load 'test_helper/bats-assert/load'
    load 'test_helper/bats-file/load'

    # Create a temporary directory for each test
    TEST_TEMP_DIR="$(temp_make)"
    mkdir -p "$TEST_TEMP_DIR/src/proj"
    cd "$TEST_TEMP_DIR/src/proj"

    # Initialize a git repository
    export GIT_CONFIG_NOSYSTEM=1
    export HOME="$TEST_TEMP_DIR"
    git config --global init.defaultBranch main
    git config --global user.email "test@example.com"
    git config --global user.name "Test User"
    git init .

    # Add hk to PATH (assuming it's installed)
    PATH="$(dirname $BATS_TEST_DIRNAME)/target/debug:$PATH"
}

teardown() {
    chmod -R u+w "$TEST_TEMP_DIR"
    temp_del "$TEST_TEMP_DIR"
}

@test "hk --version prints version" {
    run hk --version
    assert_output --regexp "^hk\ [0-9]+\.[0-9]+\.[0-9]+$"
}

@test "hk generate creates hk.toml" {
    run hk generate
    assert_file_contains hk.toml "plugin = \"end-of-file-fixer\""
}

@test "hk install creates git hooks" {
    run hk generate
    run hk install
    assert_file_exists ".git/hooks/pre-commit"
}

@test "hk format formats code" {
    echo "let x = 1" > test.js
    run hk format prettier --file test.js
    assert_file_contains test.js "let x = 1;"
}

# @test "hk run pre-commit runs on staged files" {
#     echo -n "test file without newline" > test.txt
#     run git add test.txt
#     run hk generate
#     run hk install
#     run cat -e test.txt
#     assert_output "test file without newline"
#     run git commit -m "test"
#     run cat -e test.txt
#     assert_output "test file without newline$"
# }

# @test "hk run pre-commit --all runs on all files" {
#     # Setup test files
#     echo -n "unstaged file" > unstaged.txt
#     echo -n "staged file" > staged.txt
#     git add staged.txt
    
#     # Generate and install hooks
#     hk generate
#     hk install

#     # Run pre-commit with --all
#     run hk run pre-commit --all
#     [ "$status" -eq 0 ]

#     # Verify both files were fixed
#     run tail -c1 unstaged.txt
#     [ "$output" = "" ]
#     run tail -c1 staged.txt
#     [ "$output" = "" ]
# }

# @test "hk run pre-commit --hook runs specific hook" {
#     # Setup test file
#     echo -n "test file" > test.txt
#     git add test.txt
    
#     # Generate and install hooks
#     hk generate
#     hk install

#     # Run specific hook
#     run hk run pre-commit --hook end-of-file-fixer
#     [ "$status" -eq 0 ]

#     # Verify file was fixed
#     run tail -c1 test.txt
#     [ "$output" = "" ]
# } 
