#!/usr/bin/env mise run test:bats

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
@test "hk test supports before/after hooks" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "sh -c 'echo main >> {{tmp}}/f.txt'"
        tests {
          ["pre and post mutate file"] {
            before = "sh -c 'echo before > {{tmp}}/f.txt'"
            after = "sh -c 'echo after >> {{tmp}}/f.txt'"
            files = List("{{tmp}}/f.txt")
            expect { files { ["{{tmp}}/f.txt"] = "before\nmain\nafter\n" } }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo --name "pre and post mutate file"
    assert_success
    assert_output --partial "ok - demo :: pre and post mutate file"
}

@test "hk test fails when before exits nonzero" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo should-not-run"
        tests {
          ["before fails"] {
            before = "sh -c 'exit 2'"
            expect { code = 0 }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo --name "before fails"
    assert_failure
}

@test "hk test fails when after exits nonzero" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "true"
        tests {
          ["after fails"] {
            after = "sh -c 'exit 3'"
            expect { code = 0 }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo --name "after fails"
    assert_failure
}

