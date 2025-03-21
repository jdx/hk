#!/usr/bin/env bash

_common_setup() {
    load 'test_helper/bats-support/load'
    load 'test_helper/bats-assert/load'
    load 'test_helper/bats-file/load'

    export PROJECT_ROOT="$BATS_TEST_DIRNAME/.."
    export PKL_PATH="$PROJECT_ROOT/pkl"

    # Create a temporary directory for each test
    TEST_TEMP_DIR="$(temp_make)"
    mkdir -p "$TEST_TEMP_DIR/src/proj"
    cd "$TEST_TEMP_DIR/src/proj"

    # Initialize a git repository
    export GIT_CONFIG_NOSYSTEM=1
    export HK_JOBS=2
    export HOME="$TEST_TEMP_DIR"
    git config --global init.defaultBranch main
    git config --global user.email "test@example.com"
    git config --global user.name "Test User"
    git init .

    # Add hk to PATH (assuming it's installed)
    PATH="$(dirname $BATS_TEST_DIRNAME)/target/debug:$PATH"
}

_common_teardown() {
    chmod -R u+w "$TEST_TEMP_DIR"
    temp_del "$TEST_TEMP_DIR"
}
