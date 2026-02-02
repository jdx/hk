#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk check --pr checks only files changed from default branch" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    echo "base" > base.txt
    git add .
    git commit -m "initial commit"

    # Create a feature branch with a new file
    git checkout -b feature
    echo "new" > new.txt
    git add new.txt
    git commit -m "add new.txt"

    run hk check --pr
    assert_success
    assert_output --partial "checking new.txt"
    refute_output --partial "base.txt"
}

@test "hk check --pr conflicts with --all" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git add .
    git commit -m "initial commit"

    run hk check --pr --all
    assert_failure
    assert_output --partial "cannot be used with"
}

@test "hk check --pr conflicts with --from-ref" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git add .
    git commit -m "initial commit"

    run hk check --pr --from-ref main
    assert_failure
    assert_output --partial "cannot be used with"
}
