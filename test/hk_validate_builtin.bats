#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk_validate builtin passes for a clean config" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["check"] {
    steps { ["hk_validate"] = Builtins.hk_validate }
  }
}
PKL
    git add hk.pkl
    run hk check
    assert_success
}

@test "hk_validate builtin fails when the config has warnings" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["check"] {
    steps {
      ["hk_validate"] = Builtins.hk_validate
      ["nocmd"] {}
    }
  }
}
PKL
    git add hk.pkl
    run hk validate
    assert_output --partial "warning"
    run hk check
    assert_failure
    assert_output --partial "deny-warnings"
}
