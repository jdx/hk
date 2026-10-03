#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

write_config() {
    local format_line="$1" expectation="$2"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["compiler"] {
        check = "printf 'src/main.c:2:4: warning: first line [W1]\\\\n  second line\\\\n' >&2; exit 1"
        $format_line
        tests {
          ["reports a warning"] {
            run = "check"
            expect {
              code = 1
              diagnostics {
                $expectation
              }
            }
          }
        }
      }
    }
  }
}
PKL
}

@test "hk test passes when check output parses into the expected diagnostics" {
    write_config 'diagnostic_format = "gcc"' 'new { path = "src/main.c"; line = 2; column = 4; severity = "warning"; rule = "W1"; message = "second line" }'

    run hk test --step compiler
    assert_success
    assert_output --partial "ok - compiler :: reports a warning"
}

@test "hk test fails and lists parsed diagnostics when none match" {
    write_config 'diagnostic_format = "gcc"' 'new { path = "src/main.c"; line = 3 }'

    run hk test --step compiler
    assert_failure
    assert_output --partial "no diagnostic matches"
    assert_output --partial "parsed 1 diagnostic(s)"
    assert_output --partial "src/main.c:2:4"
}

@test "TestMaker.checkDiagnostics builds a diagnostics test" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Config.pkl"
import "$PKL_PATH/builtins/test/helpers.pkl"
local const testMaker = new helpers.TestMaker { filename = "main.c" }
hooks {
  ["check"] {
    steps {
      ["compiler"] {
        check = "printf 'main.c:2:4: error: bad\\\\n' >&2; exit 1"
        diagnostic_format = "gcc"
        tests {
          ["bad file"] = testMaker.checkDiagnostics(
            "int x;",
            1,
            List(new Config.StepTestDiagnostic { path = "main.c"; line = 2; severity = "error" }),
          )
        }
      }
    }
  }
}
PKL

    run hk test --step compiler
    assert_success
    assert_output --partial "ok - compiler :: bad file"
}

@test "hk test fails when the step sets no diagnostic_format" {
    write_config '' 'new { line = 2 }'

    run hk test --step compiler
    assert_failure
    assert_output --partial "requires the step to set diagnostic_format"
}
