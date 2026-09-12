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

@test "check --quiet shows failed step output summary" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo some diagnostic && exit 1" } } }
}
EOF
    git add hk.pkl
    run hk check --quiet
    assert_failure
    assert_output --partial "some diagnostic"
}

@test "check --stats --quiet suppresses statistics" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git add hk.pkl
    run hk check --stats --quiet
    assert_success
    refute_output --partial "Statistics"
}

@test "check --stats --silent suppresses statistics" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git add hk.pkl
    run hk check --stats --silent
    assert_success
    assert_output ""
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

@test "HK_SILENT suppresses successful output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["a"] { check = "echo check-success" } } } }
EOF
    git add hk.pkl
    run env HK_SILENT=true hk check
    assert_success
    assert_output ""

    run env HK_SILENT=true hk check --stats
    assert_success
    assert_output ""
}

@test "HK_SILENT suppresses failed step summaries and preserves output file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["a"] { check = "echo check-diagnostic && exit 1" } } } }
EOF
    git add hk.pkl
    run env HK_SILENT=1 HK_OUTPUT_FILE="$PWD/output.log" hk check
    assert_failure
    refute_output --partial "check-diagnostic"
    refute_output --partial "output:"
    assert_file_contains output.log "check-diagnostic"
}

@test "HK_SILENT works with CLI output flags" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["check"] { steps { ["a"] { check = "echo check-success" } } } }
EOF
    git add hk.pkl
    run env HK_SILENT=0 hk check
    assert_success
    assert_output --partial "check-success"

    run env HK_SILENT=0 hk check --silent
    assert_success
    assert_output ""

    run env HK_SILENT=1 hk check --verbose
    assert_success
    assert_output ""
}

@test "git hooks inherit HK_SILENT" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["a"] { check = "echo hook-success; touch hook-ran" } } } }
EOF
    run env HK_SILENT=1 hk install
    assert_success
    assert_output ""
    git add hk.pkl
    run env HK_SILENT=1 git commit -m "test hook"
    assert_success
    assert_file_exist hook-ran
    refute_output --partial "hook-success"
}
