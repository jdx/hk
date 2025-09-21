#!/usr/bin/env bats

# Test cache environment variable controls

setup() {
    load 'test_helper/common_setup'
    # Don't call _common_setup yet - we'll control cache per test
}

teardown() {
    if [ -n "$TEST_TEMP_DIR" ]; then
        _common_teardown
    fi
}

@test "HK_TEST_CACHE_DISABLED=1 disables caching" {
    export HK_TEST_CACHE_DISABLED=1
    _common_setup

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

    # Cache should be disabled - pointing to temp location
    [[ "$HK_CACHE_DIR" == *"/.no-cache" ]] || [ ! -d "$HK_CACHE_DIR" ]
}

@test "HK_TEST_CACHE_DIR overrides cache location" {
    export HK_TEST_CACHE_DIR="/tmp/custom-hk-test-cache-$$"
    _common_setup

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] { shell = "echo custom cache" }
        }
    }
}
EOF

    run hk validate
    assert_success

    # Should use custom cache directory
    [ "$HK_CACHE_DIR" = "$HK_TEST_CACHE_DIR" ]
    [ -d "$HK_CACHE_DIR" ]

    # Cleanup
    rm -rf "$HK_TEST_CACHE_DIR"
}

@test "HK_TEST_CACHE_NO_CLEANUP=1 prevents stale cache cleanup" {
    export HK_TEST_CACHE_NO_CLEANUP=1
    export HK_TEST_CACHE_DIR="/tmp/no-cleanup-test-$$"
    _common_setup

    # Create a fake old cache file
    mkdir -p "$HK_CACHE_DIR"
    touch -t 202301010000 "$HK_CACHE_DIR/old-cache.json"

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

    # Old cache file should still exist (not cleaned up)
    [ -f "$HK_CACHE_DIR/old-cache.json" ]

    # Cleanup
    rm -rf "$HK_TEST_CACHE_DIR"
}

@test "default behavior enables caching" {
    # No special env vars set
    unset HK_TEST_CACHE_DISABLED
    unset HK_TEST_CACHE_DIR
    unset HK_TEST_CACHE_NO_CLEANUP
    _common_setup

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] { shell = "echo default cache" }
        }
    }
}
EOF

    run hk validate
    assert_success

    # Should use default cache location
    [[ "$HK_CACHE_DIR" == */hk-test-cache ]]
    [ -d "$HK_CACHE_DIR" ]
}
