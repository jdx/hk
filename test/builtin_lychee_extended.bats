#!/usr/bin/env bats
setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "lychee_extended Flint checker test runs" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["lychee_extended"] = Builtins.lychee_extended
    }
  }
}
PKL

    git status --short >/dev/null
    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step lychee_extended
    assert_success
    assert_output --partial "ok - lychee_extended :: check markdown without links"
    assert_output --partial "ok - lychee_extended :: remap same-repository PR links"
}
