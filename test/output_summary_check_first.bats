#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "output_summary suppressed for check when fixer runs with check_first" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["format1"] {
        check_first = true
        output_summary = "stderr"
        check = "echo 'Check1 failed' 1>&2 && exit 1"
        fix = "echo 'Fixed1' 1>&2"
        glob = List("*.txt")
      }
      ["format2"] {
        check_first = true
        output_summary = "stderr"
        check = "echo 'Check2 failed' 1>&2 && exit 1"
        fix = "echo 'Fixed2' 1>&2"
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
    # Should only show fix output in the summary, not check output
    assert_output --partial "format1 stderr:"
    assert_output --partial "Fixed1"
    assert_output --partial "format2 stderr:"
    assert_output --partial "Fixed2"
    # The summary section should not contain check output
    # Note: Check output may appear in the progress output, but not in the summary
    summary=$(echo "$output" | sed -n '/format[12] stderr:/,$p')
    echo "$summary" | grep -q "Fixed1" || fail "Summary should contain Fixed1"
    echo "$summary" | grep -q "Fixed2" || fail "Summary should contain Fixed2"
    ! echo "$summary" | grep -q "Check1 failed" || fail "Summary should not contain Check1 failed"
    ! echo "$summary" | grep -q "Check2 failed" || fail "Summary should not contain Check2 failed"
}

@test "output_summary shown for check when fixer does NOT run with check_first" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["format1"] {
        check_first = true
        output_summary = "stderr"
        check = "echo 'Check1 passed' 1>&2 && exit 0"
        fix = "echo 'Fixed1' 1>&2"
        glob = List("*.txt")
      }
      ["format2"] {
        check_first = true
        output_summary = "stderr"
        check = "echo 'Check2 passed' 1>&2 && exit 0"
        fix = "echo 'Fixed2' 1>&2"
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
    # Should show check output since fix didn't run
    assert_output --partial "Check1 passed"
    assert_output --partial "Check2 passed"
    refute_output --partial "Fixed1"
    refute_output --partial "Fixed2"
}

@test "output_summary suppressed for check_list_files when fixer runs with check_first" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["format1"] {
        check_first = true
        output_summary = "stderr"
        check_list_files = "echo 'file.txt' && echo 'Check list output' 1>&2 && exit 1"
        fix = "echo 'Fixed1' 1>&2"
        glob = List("*.txt")
      }
      ["format2"] {
        check_first = true
        output_summary = "stderr"
        check = "echo 'Check2 failed' 1>&2 && exit 1"
        fix = "echo 'Fixed2' 1>&2"
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
    # Should only show fix output in the summary, not check_list_files output
    assert_output --partial "Fixed1"
    assert_output --partial "Fixed2"
    # The summary section should not contain check output
    summary=$(echo "$output" | sed -n '/format[12] stderr:/,$p')
    echo "$summary" | grep -q "Fixed1" || fail "Summary should contain Fixed1"
    echo "$summary" | grep -q "Fixed2" || fail "Summary should contain Fixed2"
    ! echo "$summary" | grep -q "Check list output" || fail "Summary should not contain Check list output"
}

@test "output_summary suppressed for check_diff when fixer runs with check_first" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["format1"] {
        check_first = true
        output_summary = "stderr"
        check_diff = "echo 'Diff output' 1>&2 && exit 1"
        fix = "echo 'Fixed1' 1>&2"
        glob = List("*.txt")
      }
      ["format2"] {
        check_first = true
        output_summary = "stderr"
        check = "echo 'Check2 failed' 1>&2 && exit 1"
        fix = "echo 'Fixed2' 1>&2"
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
    # Should only show fix output in the summary, not check_diff output
    assert_output --partial "Fixed1"
    assert_output --partial "Fixed2"
    # The summary section should not contain check output
    summary=$(echo "$output" | sed -n '/format[12] stderr:/,$p')
    echo "$summary" | grep -q "Fixed1" || fail "Summary should contain Fixed1"
    echo "$summary" | grep -q "Fixed2" || fail "Summary should contain Fixed2"
    ! echo "$summary" | grep -q "Diff output" || fail "Summary should not contain Diff output"
}

@test "output_summary works normally without check_first" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] {
        output_summary = "stderr"
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
    # Should show check output normally
    assert_output --partial "Check output"
}
