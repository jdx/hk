#!/usr/bin/env bash

_common_setup() {
    # Optional debug tracing for CI / repro
    if [ "${BATS_DEBUG:-}" = "1" ]; then
        set -x
        export GIT_TRACE=1
        export GIT_TRACE_SETUP=1
    fi
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
    # Use CARGO_TARGET_DIR if set (e.g., by mise), otherwise use local target
    if [ -n "$CARGO_TARGET_DIR" ]; then
        PATH="$CARGO_TARGET_DIR/debug:$PATH"
    else
        PATH="$(dirname $BATS_TEST_DIRNAME)/target/debug:$PATH"
    fi

    # Show key env for debugging
    echo "[bats-debug] PATH=$PATH"
    echo "[bats-debug] which hk: $(command -v hk || echo 'not found')"
    hk --version 2>/dev/null || echo "[bats-debug] hk --version failed"

    # Enable test cache by default for better performance
    # Individual tests can override this by calling _disable_test_cache
    _enable_test_cache
}

_common_teardown() {
    chmod -R u+w "$TEST_TEMP_DIR"
    temp_del "$TEST_TEMP_DIR"
}
