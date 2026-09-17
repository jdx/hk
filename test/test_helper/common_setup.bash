#!/usr/bin/env bash

_common_setup() {
    export PROJECT_ROOT="$(dirname "${BASH_SOURCE[0]}")/../.."
    load "$PROJECT_ROOT/test/test_helper/bats-support/load"
    load "$PROJECT_ROOT/test/test_helper/bats-assert/load"
    load "$PROJECT_ROOT/test/test_helper/bats-file/load"
    load "$PROJECT_ROOT/test/test_helper/cache_setup"

    export PKL_PATH="$PROJECT_ROOT/pkl"

    # Create a temporary directory for each test
    TEST_TEMP_DIR="$(temp_make)"
    mkdir -p "$TEST_TEMP_DIR/src/proj"
    cd "$TEST_TEMP_DIR/src/proj"

    # Initialize a git repository
    export GIT_CONFIG_NOSYSTEM=1
    export HK_JOBS=2
    export MISE_INSTALLS_DIR="${MISE_INSTALLS_DIR:-$HOME/.local/share/mise/installs}"
    export XDG_CACHE_HOME="${XDG_CACHE_HOME:-$MISE_INSTALLS_DIR/.cache}"
    export XDG_DATA_HOME="${XDG_DATA_HOME:-$MISE_INSTALLS_DIR/.local/share}"
    export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
    export HOME="$TEST_TEMP_DIR"

    # Install tool user configs into the temp HOME so tools behave correctly in tests.
    export XDG_CONFIG_HOME="$HOME/.config"
    mkdir -p "$XDG_CONFIG_HOME"
    cp -r "$PROJECT_ROOT/test/test_helper/xdg_config/." "$XDG_CONFIG_HOME/"

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

    # Enable test cache by default for better performance
    # Individual tests can override this by calling _disable_test_cache
    _enable_test_cache
}

# Record `hk test` per-case durations from $output to a file under target/.
#
# `hk test` prints "ok - <step> :: <case> (<n>ms)", but callers wrap it in
# `run ... ; assert_success`, so bats only dumps those lines when the test
# FAILS. That left the suite's timing data invisible on green runs. CI uploads
# this directory so slow cases can be tracked without waiting for a failure.
#
# The durations are wall-clock per case and include any tool install the case
# triggers, so concurrent cases sharing one install each report its full cost.
_record_test_timings() {
    local name=$1
    local dir="$PROJECT_ROOT/target/test-timings"
    mkdir -p "$dir" || return 0
    # awk, not sed: BSD sed (macOS) writes a literal "t" for \t in a
    # replacement, which would collapse both columns on the platform these
    # timings matter most for.
    printf '%s\n' "$output" \
        | awk '/^ok - .* \([0-9]+ms\)$/ {
                 name = $0
                 sub(/^ok - /, "", name)
                 ms = name
                 sub(/^.*\(/, "", ms)
                 sub(/ms\)$/, "", ms)
                 sub(/ \([0-9]+ms\)$/, "", name)
                 printf "%s\t%s\n", ms, name
               }' \
        | sort -rn > "$dir/$name.tsv" || true
}

_common_teardown() {
    chmod -R u+w "$TEST_TEMP_DIR"
    temp_del "$TEST_TEMP_DIR"
}
