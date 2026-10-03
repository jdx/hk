#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "top-level jobs in hk.pkl is a real setting" {
    unset HK_JOBS
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
jobs = 3
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    run hk config get jobs
    assert_success
    assert_output "3"
    # The environment still outranks the config file.
    HK_JOBS=5 run hk config get jobs
    assert_success
    assert_output "5"
}

@test "top-level jobs in the user config applies when the project sets none" {
    unset HK_JOBS
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    mkdir -p "$HOME/.config/hk"
    cat <<EOF > "$HOME/.config/hk/config.pkl"
amends "$PKL_PATH/Config.pkl"
jobs = 2
EOF
    run hk config get jobs
    assert_success
    assert_output "2"
}

@test "project jobs wins over user config jobs" {
    unset HK_JOBS
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
jobs = 4
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    mkdir -p "$HOME/.config/hk"
    cat <<EOF > "$HOME/.config/hk/config.pkl"
amends "$PKL_PATH/Config.pkl"
jobs = 2
EOF
    run hk config get jobs
    assert_success
    assert_output "4"
}

@test "HK_STASH accepts the documented booleans" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    git add hk.pkl
    git commit -m init
    HK_STASH=true run hk check --all
    assert_success
    HK_STASH=0 run hk check --all
    assert_success
}

@test "HK_STASH with an unknown value is an error, not a panic" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    git add hk.pkl
    git commit -m init
    HK_STASH=bogus run hk check --all
    assert_failure
    assert_output --partial 'invalid HK_STASH value "bogus"'
    assert_output --partial "expected git, patch-file, none, true or false"
    refute_output --partial "panicked"
}

@test "hk config prints the settings as TOML when some are unset" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    run hk config
    assert_success
    assert_output --partial "fail_fast = true"
    refute_output --partial "invalid type: null"
}
