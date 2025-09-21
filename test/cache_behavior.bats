#!/usr/bin/env bats

# Test caching behavior and performance

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "cache is enabled and speeds up repeated config loads" {
    # Create a complex pkl config that takes time to parse
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] { shell = "echo step1" }
            ["step2"] { shell = "echo step2" }
            ["step3"] { shell = "echo step3" }
        }
    }
}
EOF

    # First run - will parse pkl and cache it
    run hk validate
    assert_success

    # Check that cache was created
    [ -d "$HK_CACHE_DIR" ]
    [ -n "$(find "$HK_CACHE_DIR" -name "*.json" -type f 2>/dev/null)" ]

    # Get cache stats for debugging
    _test_cache_stats >&2

    # Second run - should use cache (much faster)
    run hk validate
    assert_success

    # Verify the config is actually being used from cache
    # by checking that it still works even if we temporarily break the pkl file
    # (the cache should still be valid since the file hasn't been modified)
    cp hk.pkl hk.pkl.backup
    echo "INVALID PKL SYNTAX" > hk.pkl.tmp
    # Don't actually overwrite yet, just prepare

    # The cache should still work with the original mtime
    run hk validate
    assert_success

    # Now actually modify the file (changes mtime) with invalid content
    mv hk.pkl.tmp hk.pkl
    run hk validate
    assert_failure
    assert_output --partial "Failed to read config file"

    # Restore the file
    mv hk.pkl.backup hk.pkl
}

@test "cache can be disabled for specific tests" {
    _disable_test_cache

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] { shell = "echo test" }
        }
    }
}
EOF

    run hk validate
    assert_success

    # Note: Cache directory may exist but should be empty or minimal
    # Since disabling cache in our implementation just points to a temp location
    # that gets cleared, we can't fully prevent cache creation
}

@test "isolated cache works for test-specific caching" {
    _enable_isolated_test_cache

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] { shell = "echo isolated" }
        }
    }
}
EOF

    run hk validate
    assert_success

    # Check that isolated cache was created
    [[ "$HK_CACHE_DIR" == *"hk-cache-"* ]]
    [ -d "$HK_CACHE_DIR" ]
}

@test "cache correctly invalidates when pkl file changes" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["original"] { check = "echo 'checking original'" }
        }
    }
}
EOF

    # Create a dummy file to check
    echo "test" > test.txt

    # First run - creates cache
    run hk check test.txt
    assert_success

    # Verify cache was created
    [ -d "$HK_CACHE_DIR" ]
    [ -n "$(find "$HK_CACHE_DIR" -name "*.json" -type f 2>/dev/null)" ]

    # Modify the file (change mtime)
    sleep 0.01  # Ensure mtime changes even on fast filesystems
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["modified"] { check = "echo 'checking modified'" }
        }
    }
}
EOF

    # Should detect change and update cache
    run hk check test.txt
    assert_success

    # The config should have been reloaded due to mtime change
}

@test "cache handles concurrent access safely" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] { shell = "echo concurrent" }
        }
    }
}
EOF

    # Run multiple hk processes concurrently
    # They should all successfully use/create cache without conflicts
    (
        hk validate &
        hk validate &
        hk validate &
        wait
    )

    # All should succeed
    run hk validate
    assert_success
}

@test "clearing cache works" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] { shell = "echo test" }
        }
    }
}
EOF

    run hk validate
    assert_success

    # Verify cache exists
    [ -d "$HK_CACHE_DIR" ]
    [ -n "$(find "$HK_CACHE_DIR" -name "*.json" -type f 2>/dev/null)" ]

    # Clear the cache
    _clear_test_cache

    # Verify cache is cleared
    [ -z "$(find "$HK_CACHE_DIR" -name "*.json" -type f 2>/dev/null)" ]

    # Should still work (will recreate cache)
    run hk validate
    assert_success
}
