#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test runs step-defined tests" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo checking {{files}}"
        fix = "sh -c 'echo hi > out.txt'"
        tests {
          ["check stdout"] {
            run = "check"
            expect { stdout = "checking" }
          }
          ["writes file"] {
            run = "fix"
            expect { files { ["out.txt"] = "hi\n" } }
          }
        }
      }
    }
  }
}
PKL

    run hk test
    assert_success
    assert_output --partial "ok - demo :: check stdout"
    assert_output --partial "ok - demo :: writes file"
}

@test "hk test --list lists tests" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo checking {{files}}"
        tests { ["t1"] {} ["t2"] {} }
      }
    }
  }
}
PKL
    run hk test --list
    assert_success
    assert_output --partial "demo :: t1"
    assert_output --partial "demo :: t2"
}
