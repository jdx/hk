#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "fail_fast=true aborts on first failure" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
  ["check"] {
    steps {
      ["first"] {
        exclusive = true
        check = "sh -c 'echo FIRST && exit 2'"
      }
      ["second"] { check = "echo SECOND" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "test" > test.txt

    run hk check
    assert_failure
    assert_output --partial "FIRST"
    # Should not run the second step when fail_fast=true
    refute_output --partial "SECOND"
}

@test "fail_fast=false continues after failure" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
  ["check"] {
    steps {
      ["first"] {
        exclusive = true
        check = "sh -c 'echo FIRST && exit 2'"
      }
      ["second"] { check = "echo SECOND" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "test" > test.txt

    run hk check
    # Overall run still fails due to first step
    assert_failure
    assert_output --partial "FIRST"
    # With fail_fast=false, the second step should still run
    assert_output --partial "SECOND"
}
