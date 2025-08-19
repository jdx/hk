#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test runs sandboxed (cwd is tmp instead of project root)" {
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
            // Expect project root path fragment, which should fail if sandboxed
            expect { stdout = "pwd: $(pwd)/src/proj" }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step cwd
    assert_failure
    assert_output --partial "stdout:"
    assert_output --partial "pwd: $(pwd)/src/proj"
}

@test "hk test ignores step.dir during tests (still sandboxed)" {
    mkdir -p app
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["cwd_dir"] {
        dir = "app"
        check = "pwd"
        tests {
          ["prints working directory under dir"] {
            run = "check"
            // Expect project root/app path fragment, which should fail if sandboxed
            expect { stdout = "pwd: $(pwd)/src/proj/app" }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step cwd_dir
    assert_failure
    assert_output --partial "stdout:"
    assert_output --partial "pwd: $(pwd)/src/proj/app"
}
