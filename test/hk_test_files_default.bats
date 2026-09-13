#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test defaults files to globbed write keys" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        glob = "*.txt"
        exclude = "excluded.txt"
        check = "echo {{files}} && exit 1"
        tests {
          ["omits files"] {
            run = "check"
            write {
              ["{{tmp}}/test.txt"] = "content"
              ["{{tmp}}/excluded.txt"] = "content"
              ["{{tmp}}/test.config"] = "content"
            }
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

@test "hk test reports when the step's filters exclude every written file" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        glob = "*.yaml"
        check = "echo {{files}}"
        tests {
          ["writes only unmatched files"] {
            run = "check"
            write {
              ["{{tmp}}/main.tf"] = "content"
            }
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
    assert_output --partial "the step's file filters excluded all 1 file(s) written by this test"
    assert_output --partial "set the test's \`files\` explicitly"
    refute_output --partial "code="
}

@test "hk test reports the exit code when a command did run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "exit 3"
        tests {
          ["command fails"] {
            run = "check"
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
    assert_output --partial "(code=3;"
}

@test "hk test still runs a step without filters whose test writes files" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "echo ran {{files}}"
        tests {
          ["no filters"] {
            run = "check"
            write {
              ["{{tmp}}/main.tf"] = "content"
            }
            expect { code = 0; stdout = "main.tf" }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo
    assert_success
    assert_output --partial "ok - demo :: no filters"
}
