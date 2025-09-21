#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test surfaces failing tests with non-zero exit" {
    setup_with_config 'amends "'"$PKL_PATH"'/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "sh -c '"'"'echo failing >&2; exit 2'"'"'"
        tests {
          ["fails exits nonzero"] { run = "check" }
        }
      }
    }
  }
}'

    assert_hk_failure test --step demo
    assert_output --partial "not ok - demo :: fails exits nonzero"
}
