#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test surfaces failing tests with non-zero exit" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "sh -c 'echo failing >&2; exit 2'"
        tests {
          ["fails exits nonzero"] { run = "check" }
        }
      }
    }
  }
}
PKL

    run hk test --step demo
    assert_failure
    assert_output --partial "not ok - demo :: fails exits nonzero"
}
