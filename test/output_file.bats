#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "HK_OUTPUT_FILE writes failed command output to a custom path" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] {
        check = "echo CUSTOM_OUTPUT_FILE_MARKER 1>&2 && exit 1"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    output_file="$BATS_TEST_TMPDIR/nested/logs/failed-output.log"
    HK_OUTPUT_FILE="$output_file" run hk check

    assert_failure
    assert_output --partial "See $output_file for full command output"
    assert_file_exist "$output_file"
    assert_file_contains "$output_file" "CUSTOM_OUTPUT_FILE_MARKER"
    assert_file_not_exist "$HK_STATE_DIR/output.log"
}

@test "empty HK_OUTPUT_FILE uses the default location" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] {
        check = "echo EMPTY_OUTPUT_FILE_MARKER 1>&2 && exit 1"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt
    export HK_STATE_DIR="$BATS_TEST_TMPDIR/hk_state"

    HK_OUTPUT_FILE="" run hk check

    output_file="$HK_STATE_DIR/output.log"
    assert_failure
    assert_output --partial "See $output_file for full command output"
    assert_file_exist "$output_file"
    assert_file_contains "$output_file" "EMPTY_OUTPUT_FILE_MARKER"
}

@test "HK_OUTPUT_FILE writes to a bare filename" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] {
        check = "echo BARE_OUTPUT_FILE_MARKER 1>&2 && exit 1"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    HK_OUTPUT_FILE="failed-output.log" run hk check

    assert_failure
    assert_output --partial "See failed-output.log for full command output"
    assert_file_exist "failed-output.log"
    assert_file_contains "failed-output.log" "BARE_OUTPUT_FILE_MARKER"
}
