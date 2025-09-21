#!/usr/bin/env bash

# Helper functions for managing cache in tests
#
# The test framework enables caching by setting HK_CACHE=1
# This overrides the default behavior (cache disabled in debug builds)

# Enable cache for tests
_enable_test_cache() {
    # Enable caching (overrides debug build default)
    export HK_CACHE=1

    # Use a persistent cache directory in the system temp folder
    # This survives between test runs for performance
    export HK_CACHE_DIR="${BATS_TEST_TMPDIR:-/tmp}/hk-test-cache"

    # Create the cache directory if it doesn't exist
    mkdir -p "$HK_CACHE_DIR"

    # Clear stale cache entries (older than 1 day)
    if command -v find >/dev/null 2>&1; then
        find "$HK_CACHE_DIR" -type f -mtime +1 -delete 2>/dev/null || true
    fi
}

# Disable cache for specific tests
_disable_test_cache() {
    export HK_CACHE=0
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
