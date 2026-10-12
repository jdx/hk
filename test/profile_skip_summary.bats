#!/usr/bin/env bats

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

@test "profile_condition: supports or, and, and not" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["either"] {
                profile_condition = "profile('ci') or profile('slow')"
                check = "echo EITHER_RAN"
            }
            ["mixed"] {
                profile_condition = "profile('ci') and not profile('slow')"
                check = "echo MIXED_RAN"
            }
        }
    }
}
EOF2
    touch test.txt
    run hk check
    assert_success
    refute_output --partial "EITHER_RAN"
    refute_output --partial "MIXED_RAN"
    run hk check --profile ci
    assert_success
    assert_output --partial "EITHER_RAN"
    assert_output --partial "MIXED_RAN"
    run hk check --profile slow
    assert_success
    assert_output --partial "EITHER_RAN"
    refute_output --partial "MIXED_RAN"
    run hk check --profile ci --profile slow
    assert_success
    assert_output --partial "EITHER_RAN"
    refute_output --partial "MIXED_RAN"
}

@test "profile_condition: explicit disable wins and hints skip reason" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
warnings = List("missing-profiles")
hooks {
    ["check"] {
        steps {
            ["either"] {
                profile_condition = "profile('ci') or profile('slow')"
                check = "echo EITHER_RAN"
            }
        }
    }
}
EOF2
    touch test.txt
    run hk check --profile ci --profile '!ci'
    assert_success
    refute_output --partial "EITHER_RAN"
    run hk check --profile '!slow'
    assert_success
    refute_output --partial "EITHER_RAN"
    assert_output --partial "profile not enabled (ci)"
}

@test "profile_condition: hints omit profiles that are already enabled" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
warnings = List("missing-profiles")
hooks {
    ["check"] {
        steps {
            ["gated"] {
                profile_condition = "profile('ci') and not profile('slow')"
                check = "echo GATED_RAN"
            }
        }
    }
}
EOF2
    touch test.txt
    run hk check --profile ci --profile slow
    assert_success
    refute_output --partial "GATED_RAN"
    refute_output --partial "profile not enabled (ci, slow)"
    refute_output --partial "profile not enabled (slow)"
}
