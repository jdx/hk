#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hide warnings: HK_HIDE_WARNINGS=missing-profiles suppresses profile skip warning" {
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
    
    # First run without hiding warnings
    run hk check
    assert_success
    assert_output --partial "FAST TEST"
    assert_output --partial "1 step was skipped due to missing profiles: slow"
    
    # Now run with HK_HIDE_WARNINGS=missing-profiles
    HK_HIDE_WARNINGS=missing-profiles run hk check
    assert_success
    assert_output --partial "FAST TEST"
    refute_output --partial "skipped due to missing profiles"
}

@test "hide warnings: .hkrc.pkl hide_warnings suppresses profile skip warning" {
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
    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

display_skip_reasons = List("profile-not-enabled")
hide_warnings = List("missing-profiles")
EOF
    touch test.txt

    run hk check
    assert_success
    assert_output --partial "FAST TEST"
    refute_output --partial "skipped due to missing profiles"
}

@test "hide warnings: HK_HIDE_WARNINGS with multiple tags" {
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
    
    # Test with multiple warning tags (missing-profiles should be hidden)
    HK_HIDE_WARNINGS=foo,missing-profiles,bar run hk check
    assert_success
    assert_output --partial "FAST TEST"
    refute_output --partial "skipped due to missing profiles"
}

@test "hide warnings: HK_HIDE_WARNINGS with wrong tag doesn't suppress warning" {
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
    
    # Test with wrong tag - warning should still appear
    HK_HIDE_WARNINGS=wrong-tag run hk check
    assert_success
    assert_output --partial "FAST TEST"
    assert_output --partial "1 step was skipped due to missing profiles: slow"
}
