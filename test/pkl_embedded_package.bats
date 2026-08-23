#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    # The embedded package only matters when the config is actually evaluated.
    _disable_test_cache
    export HK_PKL_CACHE_DIR="$TEST_TEMP_DIR/pkl-cache"
    export HK_PKL_OFFLINE=1
}
teardown() {
    _common_teardown
}

# Write an hk.pkl pinning the package version passed in.
_write_pinned_config() {
    local version="$1"
    cat >hk.pkl <<EOF
amends "package://github.com/jdx/hk/releases/download/v${version}/hk@${version}#/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["echo"] {
        check = "echo ok"
      }
    }
  }
}
EOF
}

@test "embedded pkl package evaluates a matching config offline with a cold cache" {
    _write_pinned_config "$(hk version)"

    run hk validate
    assert_success
    assert_output --partial "is valid"
}

@test "HK_PKL_EMBEDDED=0 falls back to the network for a matching config" {
    _write_pinned_config "$(hk version)"

    HK_PKL_EMBEDDED=0 run hk validate
    assert_failure
    assert_output --partial "package is not cached and offline mode is enabled"
}

@test "a config pinning another version does not use the embedded package" {
    # 0.0.0 is never the running version, so this stays a cache miss.
    _write_pinned_config "0.0.0"

    run hk validate
    assert_failure
    assert_output --partial "hk@0.0.0.zip"
}
