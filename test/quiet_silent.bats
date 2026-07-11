#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "install --quiet suppresses output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["a"] { check = "echo hi" } } } }
EOF
    run hk install --quiet
    assert_success
    refute_output --partial "Installed"
}

@test "install --silent suppresses output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["a"] { check = "echo hi" } } } }
EOF
    run hk install --silent
    assert_success
    refute_output --partial "Installed"
}

@test "check --quiet suppresses progress and info output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git add hk.pkl
    run hk check --quiet
    assert_success
    refute_output --partial "checking"
    refute_output --partial "files"
}

@test "check --silent suppresses all output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git add hk.pkl
    run hk check --silent
    assert_success
    refute_output --partial "checking"
    refute_output --partial "files"
}

@test "check --silent suppresses failed step output summary" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo some diagnostic && exit 1" } } }
}
EOF
    git add hk.pkl
    run hk check --silent
    assert_failure
    refute_output --partial "some diagnostic"
    refute_output --partial "output:"
}

@test "init --quiet suppresses info messages" {
    run hk init --force --quiet
    assert_success
    refute_output --partial "Created"
    refute_output --partial "Detected"
}
