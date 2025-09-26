#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "combined_on_fail shows output during execution for successful commands" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["test-success"] {
        check = ">&2 echo 'REALTIME_OUTPUT' && exit 0"
        output_summary = "combined_on_fail"
      }
    }
  }
}
EOF
    run hk check
    assert_success
    # Output should appear during execution (new behavior - always show output)
    assert_output --partial "REALTIME_OUTPUT"
    # But not in summary since it succeeded with *_on_fail
    refute_output --partial "test-success stderr:"
}

@test "combined_on_fail shows output for failed commands" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["test-fail"] {
        check = "echo 'Error output' >&2 && echo 'Standard output' && exit 1"
        output_summary = "combined_on_fail"
      }
    }
  }
}
EOF
    run hk check
    assert_failure
    # Both stdout and stderr should appear on failure
    assert_output --partial "Error output"
    assert_output --partial "Standard output"
}

@test "stderr_on_fail only shows stderr on failure" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["test-fail"] {
        check = "echo 'Error output' >&2 && echo 'Standard output' && exit 1"
        output_summary = "stderr_on_fail"
      }
    }
  }
}
EOF
    run hk check
    assert_failure
    # Only stderr should appear on failure
    assert_output --partial "Error output"
    # We should still see stdout in the summary, but test that stderr is shown
}

@test "stdout_on_fail only shows stdout on failure" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["test-fail"] {
        check = "echo 'Error output' >&2 && echo 'Standard output' && exit 1"
        output_summary = "stdout_on_fail"
      }
    }
  }
}
EOF
    run hk check
    assert_failure
    # Should see standard output in summary on failure
    HK_SUMMARY_TEXT=1 run hk check
    assert_failure
    assert_output --partial "test-fail stdout:"
    assert_output --partial "Standard output"
}

@test "combined_on_fail is the default" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["test-default"] {
        check = ">&2 printf '%s\\n' 'QPZM' && exit 0"
      }
    }
  }
}
EOF
    run hk check
    assert_success
    # Default should be combined_on_fail, so output should be hidden on success
    # Check that output doesn't appear at the start of a line (may appear in command display)
    refute_output --regexp "^QPZM"
}

@test "multiple steps with mixed output_summary settings" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["always-show"] {
        check = "echo 'Always visible' >&2"
        output_summary = "stderr"
      }
      ["suppress-success"] {
        check = ">&2 printf '%s\\n' 'WKZP'"
        output_summary = "stderr_on_fail"
      }
      ["will-fail"] {
        check = "echo 'Failure message' >&2 && exit 1"
        output_summary = "stderr_on_fail"
      }
    }
  }
}
EOF
    HK_SUMMARY_TEXT=1 run hk check
    assert_failure
    # always-show should appear in summary
    assert_output --partial "always-show stderr:"
    assert_output --partial "Always visible"
    # suppress-success should not have its output appear (it succeeded)
    # Check that output doesn't appear at the start of a line (may appear in command display)
    refute_output --regexp "^WKZP"
    # will-fail should appear both real-time and in summary
    assert_output --partial "Failure message"
    assert_output --partial "will-fail stderr:"
}
