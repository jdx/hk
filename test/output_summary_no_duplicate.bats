#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "failing step output shown once, not duplicated" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] {
        output_summary = "stderr"
        check = "echo 'LINT_UNIQUE_ERR_8472' 1>&2 && exit 1"
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
    assert_output --partial "lint stderr:"
    assert_output --partial "LINT_UNIQUE_ERR_8472"
    refute_output --partial "Error running"
    # Count only in the summary section (after "lint stderr:" header)
    summary=$(echo "$output" | sed -n '/^lint stderr:/,$p')
    count=$(echo "$summary" | grep -c "LINT_UNIQUE_ERR_8472" || true)
    [ "$count" -eq 1 ] || fail "error appeared $count times in summary, expected 1"
}

@test "all failing steps output shown with fail_fast=false, no duplicates" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
  ["check"] {
    steps {
      ["lint-a"] {
        output_summary = "stderr"
        check = "echo 'ERR_AAA_1234' 1>&2 && exit 1"
        glob = List("*.txt")
      }
      ["lint-b"] {
        output_summary = "stderr"
        check = "echo 'ERR_BBB_5678' 1>&2 && exit 1"
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
    assert_output --partial "lint-a stderr:"
    assert_output --partial "lint-b stderr:"
    refute_output --partial "Error running"
    # Count only in the summary section (after first summary header)
    summary=$(echo "$output" | sed -n '/^lint-[ab] stderr:/,$p')
    count_a=$(echo "$summary" | grep -c "ERR_AAA_1234" || true)
    count_b=$(echo "$summary" | grep -c "ERR_BBB_5678" || true)
    [ "$count_a" -eq 1 ] || fail "ERR_AAA_1234 appeared $count_a times in summary, expected 1"
    [ "$count_b" -eq 1 ] || fail "ERR_BBB_5678 appeared $count_b times in summary, expected 1"
}
