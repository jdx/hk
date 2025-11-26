#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "no-files-to-process takes precedence over profile-not-enabled" {
    # Ensure there is at least one staged/tracked file so hk does not early-exit
    echo hi > SOME_FILE.txt
    git add SOME_FILE.txt

    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"

// Ensure skip reasons are printed for all cases we care about
display_skip_reasons = List("profile-not-enabled", "no-files-to-process", "condition-false")

hooks {
  ["check"] {
    steps {
      ["demo"] {
        // Require a missing profile
        profiles = List("slow")
        // Ensure this step has no files to process after filtering
        dir = "nonexistent_dir"
        check = "echo should-not-run"
      }
    }
  }
}
PKL

    run hk check
    assert_success
    assert_output --partial "skipped: no files to process"
    refute_output --partial "skipped: profile"
}

@test "condition-false takes precedence over profile-not-enabled" {
    # Ensure there is at least one staged/tracked file so hk does not early-exit
    echo hi > foo.txt
    git add foo.txt

    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List("profile-not-enabled", "no-files-to-process", "condition-false")

hooks {
  ["check"] {
    steps {
      ["demo"] {
        profiles = List("slow")
        condition = "false"
        check = "echo should-not-run"
      }
    }
  }
}
PKL

    run hk check
    assert_success
    assert_output --partial "skipped: condition is false"
    refute_output --partial "skipped: profile"
}

