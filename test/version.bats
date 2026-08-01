#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk --version prints version" {
    run hk --version
    assert_output --regexp "^hk\ [0-9]+\.[0-9]+\.[0-9]+$"
}

@test "hk check fails cleanly when min_hk_version is not satisfied" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
min_hk_version = "999.0.0"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF

    run hk check
    assert_failure
    assert_output --partial "less than the minimum required version 999.0.0"
    refute_output --partial "panicked"
}
