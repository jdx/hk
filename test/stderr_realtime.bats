#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "stderr is shown in real-time even for successful commands" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["show-info"] {
        check = "echo 'INFO: This is an info message' >&2 && echo 'Normal output' && exit 0"
        output_summary = "hide"
      }
    }
  }
}
EOF
    run hk check
    assert_success
    # The INFO message should appear in real-time output
    assert_output --partial "INFO: This is an info message"
}

@test "stderr is shown in real-time for failed commands too" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["show-error"] {
        check = "echo 'ERROR: Something went wrong' >&2 && exit 1"
        output_summary = "hide"
      }
    }
  }
}
EOF
    run hk check
    assert_failure
    # The ERROR message should appear in real-time output
    assert_output --partial "ERROR: Something went wrong"
}
