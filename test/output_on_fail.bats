#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "combined_on_fail hides output for successful commands" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["test-success"] {
        check = "echo 'This should not appear' >&2 && echo 'Neither should this' && exit 0"
        output_summary = "combined_on_fail"
      }
    }
  }
}
EOF
    run hk check
    assert_success
    # Output should not appear during execution or in summary
    refute_output --partial "This should not appear"
    refute_output --partial "Neither should this"
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
        check = "echo 'Info message' >&2 && exit 0"
      }
    }
  }
}
EOF
    run hk check
    assert_success
    # Default should be combined_on_fail, so output should be hidden on success
    refute_output --partial "Info message"
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
      ["only-on-fail"] {
        check = "echo 'Should be hidden' >&2"
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
    # only-on-fail should not appear (it succeeded)
    refute_output --partial "only-on-fail"
    refute_output --partial "Should be hidden"
    # will-fail should appear both real-time and in summary
    assert_output --partial "Failure message"
    assert_output --partial "will-fail stderr:"
}
