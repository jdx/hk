#!/usr/bin/env bats
setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "renovate_deps Flint checker tests run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["renovate_deps"] = Builtins.renovate_deps
    }
  }
}
PKL

    git status --short >/dev/null
    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step renovate_deps
    assert_success
    assert_output --partial "ok - renovate_deps :: check detects stale snapshot"
    assert_output --partial "ok - renovate_deps :: fix regenerates snapshot"
}
