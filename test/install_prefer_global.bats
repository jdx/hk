#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk install --prefer-global skips install when global hk hook is configured" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    # Simulate a prior `hk install --global`.
    git config --global hook.hk-pre-commit.command 'hk run pre-commit --from-hook "$@"'
    git config --global --replace-all hook.hk-pre-commit.event pre-commit

    run hk install --prefer-global --legacy
    assert_success
    assert_output --partial "skipping local install"
    assert_file_not_exists ".git/hooks/pre-commit"

    # Local config-based hook should also be absent.
    run git config --local --get hook.hk-pre-commit.command
    assert_failure
}

@test "hk install --prefer-global installs locally when no global hk hook is set" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    hk install --prefer-global --legacy
    assert_file_exists ".git/hooks/pre-commit"
}

@test "hk install --prefer-global conflicts with --global" {
    run hk install --prefer-global --global
    assert_failure
    assert_output --partial "cannot be used with"
}
