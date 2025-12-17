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
    export HK_CACHE=1
    # Create a pkl config
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] { shell = "echo step1" }
        }
    }
}
EOF

    # First run - will parse pkl and cache it
    run hk validate
    assert_success

    # Check that cache was created
    [ -d "$HK_CACHE_DIR" ]
    cache_file=$(find "$HK_CACHE_DIR" -name "*.json" -type f 2>/dev/null | head -1)
    [ -n "$cache_file" ]

    # Save the original mtime of hk.pkl
    if [[ "$OSTYPE" == "darwin"* ]]; then
        orig_mtime=$(stat -f %m hk.pkl)
    else
        orig_mtime=$(stat -c %Y hk.pkl)
    fi

    # Temporarily rename the pkl file - cache should still work
    mv hk.pkl hk.pkl.moved
    run hk validate
    assert_failure  # Should fail - no pkl file found

    # Restore the file with original content but broken
    echo "BROKEN SYNTAX" > hk.pkl

    # Set mtime back to original (cache thinks file unchanged)
    touch -t $(date -r "$orig_mtime" "+%Y%m%d%H%M.%S" 2>/dev/null || date -d "@$orig_mtime" "+%Y%m%d%H%M.%S") hk.pkl 2>/dev/null || true

    # Should succeed using cache (ignoring broken file content)
    run hk validate -vv
    assert_success
    assert_output --partial "config.load:config.load_project:cache.get_or_try_init: cache.hit"

    # Now update mtime to current time
    touch hk.pkl

    # Should fail now - cache invalidated, reads broken file
    run hk validate
    assert_failure
    assert_output --partial "Failed to load configuration"
}

@test "cache can be disabled with HK_CACHE=0" {
    export HK_CACHE=0

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

    # Clear any existing cache
    rm -rf "$HK_CACHE_DIR" 2>/dev/null || true

    run hk validate -vv
    assert_success
    refute_output --partial "cache.hit"

    run hk validate -vv
    assert_success
    refute_output --partial "cache.hit"
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
    sleep 0.01  # Ensure mtime changes even on fast filesystems

    # Should detect change and update cache
    run hk check -vv test.txt
    assert_success
    assert_output --partial "checking modified"
    assert_output --partial "cache.miss"
}

@test "cache handles imports" {
    export HK_CACHE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "./other.pkl"
hooks {
    ["check"] {
        steps = other.STEPS
    }
}
EOF
    cat <<EOF > other.pkl
import "$PKL_PATH/Config.pkl"
STEPS = new Mapping<String, Config.Step> {
    ["original"] { check = "echo 'checking original'" }
}
EOF

    # Create a dummy file to check
    echo "test" > test.txt

    # First run - creates cache
    run hk check test.txt
    assert_success
    assert_output --partial "checking original"

    run hk check -vv test.txt
    assert_success
    refute_output --partial "cache.miss"

    cat <<EOF > other.pkl
import "$PKL_PATH/Config.pkl"
STEPS = new Mapping<String, Config.Step> {
    ["modified"] { check = "echo 'checking modified'" }
}
EOF
    sleep 0.01  # Ensure mtime changes even on fast filesystems

    # Should detect change and update cache
    run hk check -vv test.txt
    assert_success
    assert_output --partial "checking modified"
    assert_output --partial "config.load:config.load_project:cache.get_or_try_init: cache.miss"
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
    run hk validate -vv
    assert_success
    assert_output --partial "cache.miss"
}
