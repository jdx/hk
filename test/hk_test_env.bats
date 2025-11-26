#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test supports StepTest.env and overrides step env" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo \$FOO:\$BAR"
        env { ["BAR"] = "baz" }
        tests {
          ["env overrides"] {
            run = "check"
            env { ["FOO"] = "foo"; ["BAR"] = "bar" }
            expect { stdout = "foo:bar" }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo
    assert_success
}
