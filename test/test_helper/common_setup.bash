#!/usr/bin/env bash

_common_setup() {
    load 'test_helper/bats-support/load'
    load 'test_helper/bats-assert/load'
    load 'test_helper/bats-file/load'
    load 'test_helper/cache_setup'

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

    # Only set user config if not already set (to avoid overriding existing config)
    if ! git config --global user.email >/dev/null 2>&1; then
        git config --global user.email "test@example.com"
    fi
    if ! git config --global user.name >/dev/null 2>&1; then
        git config --global user.name "Test User"
    fi

    git init .

    # Add hk to PATH (assuming it's installed)
    PATH="$(dirname $BATS_TEST_DIRNAME)/target/debug:$PATH"

    # Enable persistent test cache by default for better performance
    # Individual tests can override this by calling _disable_test_cache or _enable_isolated_test_cache
    _enable_persistent_test_cache
}

_common_teardown() {
    chmod -R u+w "$TEST_TEMP_DIR"
    temp_del "$TEST_TEMP_DIR"
}
