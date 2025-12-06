#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtins tests run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    // Include all Builtins.* steps
    steps = Builtins.toMap().toMapping()
  }
}
PKL

    PATH="$PATH":"$PROJECT_ROOT"/test/builtin_tool_stubs
    run hk test
    assert_success
    # At least the newlines builtin has a test
    assert_output --partial "ok - newlines :: fix bad file"
}
