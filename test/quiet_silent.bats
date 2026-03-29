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
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    run hk install --quiet
    assert_success
    refute_output --partial "Installed"
}

@test "install --silent suppresses output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
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
    run hk check --quiet
    assert_success
    refute_output --partial "files"
    refute_output --partial "no files to run"
}

@test "check --silent suppresses all output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    run hk check --silent
    assert_success
    refute_output --partial "files"
    refute_output --partial "no files to run"
}

@test "init --quiet suppresses info messages" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["a"] { check = "echo hi" } } } }
EOF
    run hk init --force --quiet
    assert_success
    refute_output --partial "Created"
    refute_output --partial "Detected"
}
