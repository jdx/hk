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

    # Restore the original file content with the original mtime.
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

    # Set mtime back to original (cache thinks file unchanged)
    touch -t $(date -r "$orig_mtime" "+%Y%m%d%H%M.%S" 2>/dev/null || date -d "@$orig_mtime" "+%Y%m%d%H%M.%S") hk.pkl 2>/dev/null || true

    # Should succeed using cache when content is unchanged.
    run hk validate -vv
    assert_success
    assert_output --partial "config.load:config.load_project:cache.get: cache.hit"

    # Content changes should invalidate the cache even if mtime is spoofed.
    echo "BROKEN SYNTAX" > hk.pkl
    touch -t $(date -r "$orig_mtime" "+%Y%m%d%H%M.%S" 2>/dev/null || date -d "@$orig_mtime" "+%Y%m%d%H%M.%S") hk.pkl 2>/dev/null || true

    # Should fail now - cache invalidated by content, reads broken file
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
import "./other_other.pkl"
STEPS = other_other.STEPS
EOF
    cat <<EOF > other_other.pkl
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

    cat <<EOF > other_other.pkl
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
    assert_output --partial "config.load:config.load_project:cache.get: cache.miss"
}

@test "cache invalidates when a glob import matches a new file" {
    export HK_CACHE=1
    mkdir generated

    cat <<EOF > generated/one.pkl
import "$PKL_PATH/Config.pkl"
STEPS: Mapping<String, Config.Step> = new {
    ["one"] { check = "echo checking one" }
}
EOF

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import* "generated/*.pkl" as generated
hooks {
    ["check"] {
        steps = new Mapping<String, Step> {
            for (_, mod in generated) {
                ...mod.STEPS
            }
        }
    }
}
EOF

    echo "test" > test.txt

    # First run - creates cache
    run hk check test.txt
    assert_success
    assert_output --partial "checking one"

    # Unchanged config still uses the cache
    run hk check -vv test.txt
    assert_success
    refute_output --partial "config.load:config.load_project:cache.get: cache.miss"

    # A brand new file matching the glob must be picked up without touching
    # hk.pkl or clearing the cache
    cat <<EOF > generated/two.pkl
import "$PKL_PATH/Config.pkl"
STEPS: Mapping<String, Config.Step> = new {
    ["two"] { check = "echo checking two" }
}
EOF

    run hk check -vv test.txt
    assert_success
    assert_output --partial "checking one"
    assert_output --partial "checking two"
    assert_output --partial "cache.imports_changed"

    # ...and the refreshed config is itself cached
    run hk check -vv test.txt
    assert_success
    assert_output --partial "checking two"
    refute_output --partial "config.load:config.load_project:cache.get: cache.miss"

    # Removing a matched file invalidates the cache too
    rm generated/two.pkl
    run hk check test.txt
    assert_success
    assert_output --partial "checking one"
    refute_output --partial "checking two"
}

@test "cache invalidates when an imported module gains a glob import" {
    export HK_CACHE=1
    mkdir generated

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "./other.pkl"
hooks {
    ["check"] { steps = other.STEPS }
}
EOF

    # other.pkl starts without any glob import
    cat <<EOF > other.pkl
import "$PKL_PATH/Config.pkl"
STEPS: Mapping<String, Config.Step> = new {
    ["plain"] { check = "echo checking plain" }
}
EOF

    cat <<EOF > generated/one.pkl
import "$PKL_PATH/Config.pkl"
STEPS: Mapping<String, Config.Step> = new {
    ["one"] { check = "echo checking one" }
}
EOF

    echo "test" > test.txt

    run hk check test.txt
    assert_success
    assert_output --partial "checking plain"

    # other.pkl gains a glob import; hk.pkl is never touched, so the imports
    # cache (keyed on hk.pkl alone) has to notice the edit on its own
    cat <<EOF > other.pkl
import "$PKL_PATH/Config.pkl"
import* "generated/*.pkl" as generated
STEPS: Mapping<String, Config.Step> = new {
    for (_, mod in generated) {
        ...mod.STEPS
    }
}
EOF

    run hk check test.txt
    assert_success
    assert_output --partial "checking one"
    refute_output --partial "checking plain"

    # ...and a file added under that newly discovered pattern must be picked up
    cat <<EOF > generated/two.pkl
import "$PKL_PATH/Config.pkl"
STEPS: Mapping<String, Config.Step> = new {
    ["two"] { check = "echo checking two" }
}
EOF

    run hk check test.txt
    assert_success
    assert_output --partial "checking one"
    assert_output --partial "checking two"
}

@test "cache invalidates when an imported module gains a new import" {
    export HK_CACHE=1

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "./other.pkl"
hooks {
    ["check"] { steps = other.STEPS }
}
EOF
    cat <<EOF > other.pkl
import "$PKL_PATH/Config.pkl"
STEPS: Mapping<String, Config.Step> = new {
    ["plain"] { check = "echo checking plain" }
}
EOF

    echo "test" > test.txt

    run hk check test.txt
    assert_success
    assert_output --partial "checking plain"

    # other.pkl starts importing a file the recorded module graph never saw
    cat <<EOF > third.pkl
import "$PKL_PATH/Config.pkl"
STEPS: Mapping<String, Config.Step> = new {
    ["third"] { check = "echo checking third" }
}
EOF
    cat <<EOF > other.pkl
import "$PKL_PATH/Config.pkl"
import "./third.pkl"
STEPS = third.STEPS
EOF

    run hk check test.txt
    assert_success
    assert_output --partial "checking third"

    # editing that newly imported file must now invalidate the cache
    cat <<EOF > third.pkl
import "$PKL_PATH/Config.pkl"
STEPS: Mapping<String, Config.Step> = new {
    ["third"] { check = "echo checking third edited" }
}
EOF

    run hk check test.txt
    assert_success
    assert_output --partial "checking third edited"
}

@test "resolved config cache is shared across identical local configs" {
    export HK_CACHE=1

    mkdir repo1 repo2
    for repo in repo1 repo2; do
        cat <<EOF > "$repo/hk.pkl"
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["shared"] { check = "echo shared" }
        }
    }
}
EOF
    done

    run bash -c "cd repo1 && hk validate -vv"
    assert_success
    assert_output --partial "cache.miss"
    resolved_count=$(find "$HK_CACHE_DIR/configs" -name "resolved-config-*.json" -type f | wc -l | tr -d ' ')
    [ "$resolved_count" -eq 1 ]

    run bash -c "cd repo2 && hk validate -vv"
    assert_success
    assert_output --partial "cache.hit"
    resolved_count=$(find "$HK_CACHE_DIR/configs" -name "resolved-config-*.json" -type f | wc -l | tr -d ' ')
    [ "$resolved_count" -eq 1 ]
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

@test "config cache is keyed on the env vars the config reads" {
    export HK_CACHE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["env"] { check = "echo value=[\(read?("env:HK_TEST_VAR") ?? "unset")]" }
        }
    }
}
EOF
    echo "test" > test.txt

    run hk check test.txt
    assert_success
    assert_output --partial "value=[unset]"

    HK_TEST_VAR=one run hk check test.txt
    assert_success
    assert_output --partial "value=[one]"

    HK_TEST_VAR=one run hk check -vv test.txt
    assert_success
    assert_output --partial "value=[one]"
    assert_output --partial "config.load:config.load_project:cache.get: cache.hit"

    # An empty value is not the same as an unset one
    HK_TEST_VAR= run hk check test.txt
    assert_success
    assert_output --partial "value=[]"

    run hk check -vv test.txt
    assert_success
    assert_output --partial "value=[unset]"
    assert_output --partial "config.load:config.load_project:cache.get: cache.hit"
}

@test "config cache tracks env vars read by imported and amended modules" {
    export HK_CACHE=1
    cat <<EOF > base.pkl
amends "$PKL_PATH/Config.pkl"
import "./other.pkl"
hooks {
    ["check"] {
        steps {
            ["env"] {
                check = "echo base=[\(read?("env:HK_TEST_BASE") ?? "unset")] other=[\(other.VALUE)]"
            }
        }
    }
}
EOF
    cat <<EOF > other.pkl
VALUE = read?("env:HK_TEST_OTHER") ?? "unset"
EOF
    cat <<EOF > hk.pkl
amends "./base.pkl"
EOF
    echo "test" > test.txt

    HK_TEST_BASE=one HK_TEST_OTHER=two run hk check test.txt
    assert_success
    assert_output --partial "base=[one] other=[two]"

    HK_TEST_OTHER=two run hk check test.txt
    assert_success
    assert_output --partial "base=[unset] other=[two]"

    HK_TEST_BASE=one run hk check test.txt
    assert_success
    assert_output --partial "base=[one] other=[unset]"
}

@test "config cache follows env vars that only some evaluations read" {
    export HK_CACHE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
local mode = read?("env:HK_TEST_MODE") ?? "plain"
local value = if (mode == "env") read?("env:HK_TEST_VAR") ?? "unset" else "plain"
hooks {
    ["check"] {
        steps {
            ["env"] { check = "echo value=[\(value)]" }
        }
    }
}
EOF
    echo "test" > test.txt

    HK_TEST_VAR=one run hk check test.txt
    assert_success
    assert_output --partial "value=[plain]"

    HK_TEST_MODE=env HK_TEST_VAR=one run hk check test.txt
    assert_success
    assert_output --partial "value=[one]"

    HK_TEST_MODE=env HK_TEST_VAR=two run hk check test.txt
    assert_success
    assert_output --partial "value=[two]"

    HK_TEST_VAR=two run hk check test.txt
    assert_success
    assert_output --partial "value=[plain]"

    HK_TEST_MODE=env HK_TEST_VAR=one run hk check -vv test.txt
    assert_success
    assert_output --partial "value=[one]"
    assert_output --partial "config.load:config.load_project:cache.get: cache.hit"
}

@test "config cache keys a read of an env var the config exports on the value read" {
    export HK_CACHE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
env {
    ["HK_TEST_VAR"] = "exported"
}
hooks {
    ["check"] {
        steps {
            ["env"] { check = "echo value=[\(read?("env:HK_TEST_VAR") ?? "unset")]" }
        }
    }
}
EOF
    echo "test" > test.txt

    run hk check test.txt
    assert_success
    assert_output --partial "value=[unset]"

    # Exporting the var must not file the entry above under its exported value
    HK_TEST_VAR=exported run hk check test.txt
    assert_success
    assert_output --partial "value=[exported]"
}

@test "subproject config sees env vars exported by the root config" {
    export HK_CACHE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("sub")
env {
    ["HK_TEST_VAR"] = "exported"
}
hooks {
    ["check"] {}
}
EOF
    mkdir sub
    cat <<EOF > sub/hk.pkl
amends "$PKL_PATH/Config.pkl"
steps {
    ["env"] { check = "echo value=[\(read?("env:HK_TEST_VAR") ?? "unset")]" }
}
EOF
    echo "test" > sub/test.txt

    run hk check sub/test.txt
    assert_success
    assert_output --partial "value=[exported]"

    HK_TEST_VAR=other run hk check sub/test.txt
    assert_success
    assert_output --partial "value=[exported]"
}

@test "hk.local.pkl env reads key the config cache" {
    export HK_CACHE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
EOF
    cat <<EOF > hk.local.pkl
amends "./hk.pkl"
hooks {
    ["check"] {
        steps {
            ["env"] { check = "echo value=[\(read?("env:HK_TEST_VAR") ?? "unset")]" }
        }
    }
}
EOF
    echo "test" > test.txt

    HK_TEST_VAR=one run hk check test.txt
    assert_success
    assert_output --partial "value=[one]"

    run hk check test.txt
    assert_success
    assert_output --partial "value=[unset]"
}

@test "config cache stays correct when the recorded env var names are lost" {
    export HK_CACHE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["env"] { check = "echo value=[\(read?("env:HK_TEST_VAR") ?? "unset")]" }
        }
    }
}
EOF
    echo "test" > test.txt

    HK_TEST_VAR=one run hk check test.txt
    assert_success
    run hk check test.txt
    assert_success

    find "$HK_CACHE_DIR/configs" -name "*-imports-*.json" -delete

    HK_TEST_VAR=one run hk check test.txt
    assert_success
    assert_output --partial "value=[one]"

    run hk check test.txt
    assert_success
    assert_output --partial "value=[unset]"
}

@test "config cache keeps env var names across re-analysis of imports" {
    export HK_CACHE=1
    mkdir generated
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import* "generated/*.pkl" as generated
hooks {
    ["check"] {
        steps {
            ["env"] { check = "echo value=[\(read?("env:HK_TEST_VAR") ?? "unset")] count=\(generated.length)" }
        }
    }
}
EOF
    echo "test" > test.txt

    HK_TEST_VAR=one run hk check test.txt
    assert_success
    assert_output --partial "value=[one] count=0"

    touch generated/one.pkl
    HK_TEST_VAR=one run hk check -vv test.txt
    assert_success
    assert_output --partial "value=[one] count=1"
    assert_output --partial "cache.imports_changed"

    # Back to the first module graph: its entry is still keyed on HK_TEST_VAR
    rm generated/one.pkl
    HK_TEST_VAR=one run hk check -vv test.txt
    assert_success
    assert_output --partial "value=[one] count=0"
    assert_output --partial "config.load:config.load_project:cache.get: cache.hit"
}
