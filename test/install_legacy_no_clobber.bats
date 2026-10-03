#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] { steps { ["a"] { check = "true" } } }
    ["pre-push"] { steps { ["a"] { check = "true" } } }
}
EOF
}

teardown() {
    _common_teardown
}

@test "legacy install refuses to overwrite a hook hk did not write" {
    printf '#!/bin/sh\necho mine\n' > .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit

    run hk install --legacy
    assert_failure
    assert_output --partial "pre-commit is a hook that hk did not write"
    assert_output --partial "--force"
    run cat .git/hooks/pre-commit
    assert_output --partial "echo mine"
}

@test "legacy install writes nothing when one of several targets is refused" {
    printf '#!/bin/sh\necho mine\n' > .git/hooks/pre-push
    chmod +x .git/hooks/pre-push

    run hk install --legacy
    assert_failure
    assert_output --partial "pre-push is a hook that hk did not write"
    assert_file_not_exists .git/hooks/pre-commit
    run cat .git/hooks/pre-push
    assert_output --partial "echo mine"
}

@test "legacy install does not remove existing hk shims when it refuses" {
    hk install --legacy
    assert_file_exists .git/hooks/pre-commit
    rm .git/hooks/pre-push
    printf '#!/bin/sh\necho mine\n' > .git/hooks/pre-push

    run hk install --legacy
    assert_failure
    run cat .git/hooks/pre-commit
    assert_output --partial "hk run pre-commit"
}

@test "legacy install refuses to write through a symlink" {
    mkdir scripts
    printf '#!/bin/sh\necho tracked\n' > scripts/hook.sh
    chmod +x scripts/hook.sh
    ln -s ../../scripts/hook.sh .git/hooks/pre-commit

    run hk install --legacy
    assert_failure
    assert_output --partial "pre-commit is a symlink"
    run cat scripts/hook.sh
    assert_output --partial "echo tracked"
    refute_output --partial "hk run"
    [ -L .git/hooks/pre-commit ]
}

@test "legacy install rewrites an existing hk shim" {
    hk install --legacy
    run hk install --legacy
    assert_success
    run cat .git/hooks/pre-commit
    assert_output --partial 'run pre-commit --from-hook "$@"'
}

@test "legacy install --force replaces a foreign hook" {
    printf '#!/bin/sh\necho mine\n' > .git/hooks/pre-commit
    chmod +x .git/hooks/pre-commit

    run hk install --legacy --force
    assert_success
    run cat .git/hooks/pre-commit
    assert_output --partial "hk run pre-commit"
    refute_output --partial "echo mine"
}

@test "legacy install --force replaces a symlink without touching its target" {
    mkdir scripts
    printf '#!/bin/sh\necho tracked\n' > scripts/hook.sh
    chmod +x scripts/hook.sh
    ln -s ../../scripts/hook.sh .git/hooks/pre-commit

    run hk install --legacy --force
    assert_success
    [ ! -L .git/hooks/pre-commit ]
    run cat .git/hooks/pre-commit
    assert_output --partial "hk run pre-commit"
    run cat scripts/hook.sh
    assert_output --partial "echo tracked"
    refute_output --partial "hk run"
}

@test "legacy install --force replaces a dangling symlink without creating its target" {
    mkdir scripts
    ln -s ../../scripts/missing.sh .git/hooks/pre-commit

    run hk install --legacy --force
    assert_success
    [ ! -L .git/hooks/pre-commit ]
    assert_file_not_exists scripts/missing.sh
    run cat .git/hooks/pre-commit
    assert_output --partial "hk run pre-commit"
}
