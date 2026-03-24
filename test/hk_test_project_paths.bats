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

@test "hk absolute binary path resolves builtin hk util subcommands" {
    local abs_hk
    if [ -n "$CARGO_TARGET_DIR" ]; then
        abs_hk="$CARGO_TARGET_DIR/debug/hk"
    else
        abs_hk="$PROJECT_ROOT/target/debug/hk"
    fi
    [ -x "$abs_hk" ]

    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["check"] {
    steps {
      ["mixed-endings"] = Builtins.mixed_line_ending
    }
  }
}
PKL

    printf "line1\r\nline2\n" > test.txt

    run env PATH="/usr/bin:/bin" "$abs_hk" check
    assert_failure
    assert_output --partial "test.txt"
}
