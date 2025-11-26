#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test defaults files to write keys" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo {{files}} && exit 1"
        tests {
          ["omits files"] {
            run = "check"
            write { ["{{tmp}}/test.txt"] = "content" }
            expect { code = 0 }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo
    assert_failure
    assert_output --partial "/test.txt"
}
