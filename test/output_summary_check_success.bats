#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# NB: these assert on the end-of-run summary block ("<step> stderr:") rather than
# the message text, which also appears in the transient progress lines.

@test "check_diff informational stderr is hidden when the check passes" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["format"] {
        check_diff = "echo '1 file did not need formatting' 1>&2"
        fix = "echo 'Fixed' 1>&2"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    HK_SUMMARY_TEXT=1 run hk check
    assert_success
    refute_output --partial "format stderr:"
}

@test "check_list_files informational stderr is hidden when the check passes" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["format"] {
        check_list_files = "echo 'all files are formatted' 1>&2"
        fix = "echo 'Fixed' 1>&2"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    HK_SUMMARY_TEXT=1 run hk check
    assert_success
    refute_output --partial "format stderr:"
}

@test "check_diff informational stderr is hidden when check_first passes during fix" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["format"] {
        check_first = true
        check_diff = "echo '1 file did not need formatting' 1>&2"
        fix = "echo 'Fixed' 1>&2"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    HK_SUMMARY_TEXT=1 run hk fix
    assert_success
    refute_output --partial "format stderr:"
    # The fixer never runs because check_diff reported nothing to fix
    refute_output --partial "Fixed"
}

@test "check_diff stderr is still shown when the check fails" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["format"] {
        check_diff = "echo 'file.txt is not formatted' 1>&2 && exit 1"
        fix = "echo 'Fixed' 1>&2"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    HK_SUMMARY_TEXT=1 run hk check
    assert_failure
    assert_output --partial "format stderr:"
    summary=$(echo "$output" | sed -n '/format stderr:/,$p')
    echo "$summary" | grep -q "file.txt is not formatted" \
        || fail "Summary should contain the check_diff failure output"
}

@test "check_diff stderr can be opted back in with output_summary combined" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["format"] {
        output_summary = "combined"
        check_diff = "echo '1 file did not need formatting' 1>&2"
        fix = "echo 'Fixed' 1>&2"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    HK_SUMMARY_TEXT=1 run hk check
    assert_success
    assert_output --partial "format output:"
    summary=$(echo "$output" | sed -n '/format output:/,$p')
    echo "$summary" | grep -q "1 file did not need formatting" \
        || fail "Summary should contain the check_diff informational output"
}

@test "plain check stderr is still shown when the check passes" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] {
        check = "echo 'Check output' 1>&2"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    HK_SUMMARY_TEXT=1 run hk check
    assert_success
    assert_output --partial "lint stderr:"
    summary=$(echo "$output" | sed -n '/lint stderr:/,$p')
    echo "$summary" | grep -q "Check output" \
        || fail "Summary should contain the check output"
}
