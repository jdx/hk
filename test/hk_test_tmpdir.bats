#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test tmpdir=true forces sandbox even without {{tmp}}" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["cwd"] {
        check = #"echo "pwd: \$(pwd)" && exit 1"#
        tests {
          ["runs in tmpdir"] {
            run = "check"
            tmpdir = true
            // Should NOT be the project root since tmpdir=true
            expect { code = 0 }
          }
        }
      }
    }
  }
}
PKL

    PROJECT_ROOT="$(pwd)"
    run hk test --step cwd
    assert_failure
    assert_output --partial "pwd: "
    refute_output --partial "pwd: $PROJECT_ROOT"
}

@test "hk test tmpdir defaults to false when {{tmp}} not used" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["cwd"] {
        check = #"echo "pwd: \$(pwd)" && exit 1"#
        tests {
          ["runs in repo root"] {
            run = "check"
            expect { stdout = "pwd: $(pwd)" }
          }
        }
      }
    }
  }
}
PKL

    PROJECT_ROOT="$(pwd)"
    run hk test --step cwd
    assert_failure
    assert_output --partial "pwd: $PROJECT_ROOT"
}

@test "hk test tmpdir=false forces repo root even with {{tmp}} in files" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["cwd"] {
        check = #"echo "pwd: \$(pwd)" && exit 1"#
        tests {
          ["runs in repo root"] {
            run = "check"
            tmpdir = false
            files = List("{{tmp}}/dummy.txt")
          }
        }
      }
    }
  }
}
PKL

    PROJECT_ROOT="$(pwd)"
    run hk test --step cwd
    assert_failure
    assert_output --partial "pwd: $PROJECT_ROOT"
}

@test "hk test tmpdir defaults to true when {{tmp}} used" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = #"echo "pwd: \$(pwd)" && exit 1"#
        tests {
          ["reads from tmpdir"] {
            run = "check"
            write { ["{{tmp}}/test.txt"] = "hello" }
          }
        }
      }
    }
  }
}
PKL

    PROJECT_ROOT="$(pwd)"
    run hk test --step demo
    assert_failure
    assert_output --partial "pwd: "
    refute_output --partial "pwd: $PROJECT_ROOT"
}
