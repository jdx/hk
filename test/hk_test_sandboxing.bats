#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test defaults to project root when {{tmp}} not used" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["cwd"] {
        check = #"echo "pwd: \$(pwd)""#
        tests {
          ["prints working directory"] {
            run = "check"
            // Expect project root
            expect { stdout = "pwd: $(pwd)" }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step cwd
    assert_success
}

@test "hk test ignores step.dir during tests (not sandboxed)" {
    mkdir -p app
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["cwd_dir"] {
        dir = "app"
        check = #"echo "pwd: \$(pwd)""#
        tests {
          ["prints working directory under dir"] {
            run = "check"
            // step.dir is ignored during tests, so still expect project root
            expect { stdout = "pwd: $(pwd)" }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step cwd_dir
    assert_success
}
