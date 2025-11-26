#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    unset HK_HIDE_WARNINGS
}

teardown() {
    _common_teardown
}

@test "profile skip summary: shows helpful message for profile-not-enabled steps" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
warnings = List("missing-profiles")
hooks {
    ["check"] {
        steps {
            ["slow-test"] {
                profiles = List("slow")
                check = "echo 'SLOW TEST'"
            }
            ["fast-test"] {
                check = "echo 'FAST TEST'"
            }
        }
    }
}
EOF
    touch test.txt
    run hk check
    assert_success
    assert_output --partial "FAST TEST"
    refute_output --partial "SLOW TEST"
    assert_output --partial "⇢ slow-test – skipped: profile not enabled (slow)"
    assert_output --partial "1 step was skipped due to missing profiles: slow"
    assert_output --partial "To enable these steps, use --slow or set HK_PROFILE=slow."
    assert_output --partial "To hide this warning: set HK_HIDE_WARNINGS=missing-profiles"
}

@test "profile skip summary: shows message for multiple profile-skipped steps" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
warnings = List("missing-profiles")
hooks {
    ["check"] {
        steps {
            ["slow-test"] {
                profiles = List("slow")
                check = "echo 'SLOW TEST'"
            }
            ["extra-test"] {
                profiles = List("extra")
                check = "echo 'EXTRA TEST'"
            }
            ["fast-test"] {
                check = "echo 'FAST TEST'"
            }
        }
    }
}
EOF
    touch test.txt
    run hk check
    assert_success
    assert_output --partial "FAST TEST"
    refute_output --partial "SLOW TEST"
    refute_output --partial "EXTRA TEST"
    assert_output --partial "2 steps were skipped due to missing profiles"
}

@test "profile skip summary: shows git-specific message for pre-commit hook" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
warnings = List("missing-profiles")
hooks {
    ["pre-commit"] {
        steps {
            ["slow-test"] {
                profiles = List("slow")
                check = "echo 'SLOW TEST'"
            }
            ["fast-test"] {
                check = "echo 'FAST TEST'"
            }
        }
    }
}
EOF
    git init >/dev/null 2>&1
    touch test.txt
    git add test.txt
    run hk run pre-commit
    assert_success
    assert_output --partial "FAST TEST"
    refute_output --partial "SLOW TEST"
    assert_output --partial "1 step was skipped due to missing profiles: slow"
    assert_output --partial "To enable these steps, set HK_PROFILE=slow environment variable or run hk fix --slow --from-ref=main"
    assert_output --partial "To hide this warning: set HK_HIDE_WARNINGS=missing-profiles"
    refute_output --partial "--profile"
}

@test "profile skip summary: no message when no profile steps are skipped" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test"] {
                check = "echo 'TEST'"
            }
        }
    }
}
EOF
    touch test.txt
    run hk check
    assert_success
    assert_output --partial "TEST"
    refute_output --partial "skipped due to missing profiles"
}
