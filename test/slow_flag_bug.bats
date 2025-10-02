#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "check --slow should enable slow profile for setup-pnpm-install" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["setup-pnpm-install"] {
                profiles = List("slow")
                check = "echo 'SETUP PNPM INSTALL'"
            }
            ["fast-test"] {
                check = "echo 'FAST TEST'"
            }
        }
    }
}
EOF
    echo "test" > test.js
    run hk check --slow test.js
    assert_success
    assert_output --partial "SETUP PNPM INSTALL"
    assert_output --partial "FAST TEST"
    refute_output --partial "skipped: profile not enabled (slow)"
}

@test "check --slow should enable slow profile" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["eslint-web-bazel"] {
                profiles = List("slow")
                check = "echo 'ESLINT WEB BAZEL'"
            }
            ["prettier"] {
                check = "echo 'PRETTIER'"
            }
        }
    }
}
EOF
    echo "test" > test.js
    run hk check --slow test.js
    assert_success
    assert_output --partial "ESLINT WEB BAZEL"
    assert_output --partial "PRETTIER"
    refute_output --partial "skipped: profile not enabled (slow)"
}

@test "--slow flag adds slow to enabled profiles" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["slow-step"] {
                profiles = List("slow")
                check = "echo 'SLOW STEP RAN'"
            }
        }
    }
}
EOF
    echo "test" > test.js
    run hk check --slow test.js
    assert_success
    assert_output --partial "SLOW STEP RAN"
}
