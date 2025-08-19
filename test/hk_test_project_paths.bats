#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk test can use absolute project paths" {
    # hk.pkl defines a step that reads a file via absolute path under the project
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["demo"] {
        check = "cat {{ files }}"
        tests {
          ["uses project file"] {
            write { ["{{root}}/project.txt"] = "hello from project" }
            files = List("{{root}}/project.txt")
            run = "check"
            expect { stdout = "hello from project" }
          }
        }
      }
    }
  }
}
PKL

    run hk test --step demo
    assert_success
    assert_output --partial "ok - demo :: uses project file"
}
