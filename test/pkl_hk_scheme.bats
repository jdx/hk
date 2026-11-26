#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    # The hk: modules only matter when the config is actually evaluated.
    _disable_test_cache
    export HK_PKL_CACHE_DIR="$TEST_TEMP_DIR/pkl-cache"
    export HK_PKL_OFFLINE=1
}
teardown() {
    _common_teardown
}

_write_hk_scheme_config() {
    cat >hk.pkl <<'PKL'
amends "hk:/Config.pkl"
import "hk:/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["prettier"] = Builtins.prettier
      ["echo"] {
        check = "echo ok"
      }
    }
  }
}
PKL
}

@test "hk: modules come from the running hk, offline with a cold cache" {
    _write_hk_scheme_config

    run hk validate
    assert_success
    assert_output --partial "is valid"
}

@test "an hk: module hk does not have is an error naming it" {
    cat >hk.pkl <<'PKL'
amends "hk:/Config.pkl"
import "hk:/Nope.pkl"

hooks {
  ["check"] {
    steps {
      ["echo"] {
        check = Nope.command
      }
    }
  }
}
PKL

    run hk validate
    assert_failure
    assert_output --partial "hk:/Nope.pkl"
}

@test "pkl reads hk: modules through hk read" {
    if ! command -v pkl >/dev/null 2>&1; then
        skip "pkl is not installed"
    fi
    _write_hk_scheme_config

    run pkl eval --format json --external-module-reader "hk=$(command -v hk) read" hk.pkl
    assert_success
    assert_output --partial '"echo ok"'
    assert_output --partial 'prettier'
}

@test "hk read exits when stdin closes" {
    run hk read </dev/null
    assert_success
    assert_output ""
}
