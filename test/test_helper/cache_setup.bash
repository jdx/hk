#!/usr/bin/env bash

# Helper functions for managing cache in tests

# Enable persistent cache for tests (shared across test runs)
# This significantly speeds up tests by reusing parsed pkl configs
_enable_persistent_test_cache() {
    # Use a persistent cache directory in the system temp folder
    # This survives between test runs but is still in a temp location
    export HK_CACHE_DIR="${BATS_TEST_TMPDIR:-/tmp}/hk-test-cache"

    # Create the cache directory if it doesn't exist
    mkdir -p "$HK_CACHE_DIR"

    # Optionally clear stale cache entries (older than 1 day)
    if command -v find >/dev/null 2>&1; then
        find "$HK_CACHE_DIR" -type f -mtime +1 -delete 2>/dev/null || true
    fi
}

# Enable isolated cache for a specific test file
# Useful when tests might interfere with each other's cache
_enable_isolated_test_cache() {
    # Use a cache directory specific to this test file
    local test_file_hash=$(echo "$BATS_TEST_FILENAME" | shasum -a 256 | cut -d' ' -f1 | head -c 8)
    export HK_CACHE_DIR="${TEST_TEMP_DIR:-/tmp}/hk-cache-${test_file_hash}"
    mkdir -p "$HK_CACHE_DIR"
}

# Disable cache completely (force fresh pkl evaluation every time)
_disable_test_cache() {
    # Point cache to /dev/null or a directory we immediately delete
    export HK_CACHE_DIR="$TEST_TEMP_DIR/.no-cache"
    rm -rf "$HK_CACHE_DIR" 2>/dev/null || true
}

# Clear the test cache
_clear_test_cache() {
    if [ -n "$HK_CACHE_DIR" ] && [ -d "$HK_CACHE_DIR" ]; then
        rm -rf "$HK_CACHE_DIR"/*
    fi
}

# Get cache statistics (useful for debugging)
_test_cache_stats() {
    if [ -n "$HK_CACHE_DIR" ] && [ -d "$HK_CACHE_DIR" ]; then
        local count=$(find "$HK_CACHE_DIR" -type f -name "*.json" 2>/dev/null | wc -l)
        local size=$(du -sh "$HK_CACHE_DIR" 2>/dev/null | cut -f1)
        echo "Cache: $count files, $size in $HK_CACHE_DIR"
    else
        echo "Cache: disabled or empty"
    fi
}
