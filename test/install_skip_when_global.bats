#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

_write_hk_pkl() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
}

_install_global_hook() {
    git config --global hook.hk-pre-commit.command 'hk run pre-commit --from-hook "$@"'
    git config --global --replace-all hook.hk-pre-commit.event pre-commit
}

@test "hk install skips local install when global hk hook is configured" {
    _write_hk_pkl
    _install_global_hook

    run hk install --legacy
    assert_success
    assert_output --partial "skipping local install"
    assert_file_not_exists ".git/hooks/pre-commit"

    # No local config-based hook either.
    run git config --local --get hook.hk-pre-commit.command
    assert_failure
}

@test "hk install installs locally when no global hk hook is set" {
    _write_hk_pkl

    hk install --legacy
    assert_file_exists ".git/hooks/pre-commit"
}

@test "hk install --force-local installs locally even when global hk hook is configured" {
    _write_hk_pkl
    _install_global_hook

    hk install --legacy --force-local
    assert_file_exists ".git/hooks/pre-commit"
}

@test "hk install cleans up stale local shims when global hk hook is now configured" {
    _write_hk_pkl

    # First, install per-repo with no global config.
    hk install --legacy
    assert_file_exists ".git/hooks/pre-commit"

    # Now the user adds a global install. A subsequent `hk install` should
    # remove the stale local shim so hk doesn't double-fire.
    _install_global_hook

    run hk install --legacy
    assert_success
    assert_output --partial "removed 1 stale local hook"
    assert_file_not_exists ".git/hooks/pre-commit"
}

@test "hk install cleans up stale local config-based hooks when global is now configured" {
    _write_hk_pkl

    # Per-repo install with config-based hooks (requires git >= 2.54).
    if ! git version | awk '{split($3,v,"."); exit !(v[1]>2 || (v[1]==2 && v[2]>=54))}'; then
        skip "git 2.54+ required for config-based hooks"
    fi

    hk install
    run git config --local --get hook.hk-pre-commit.command
    assert_success

    # User adds a global install — next `hk install` should clean up local.
    _install_global_hook

    run hk install
    assert_success
    assert_output --partial "removed 1 stale local hook"

    run git config --local --get hook.hk-pre-commit.command
    assert_failure
}

@test "hk install --force-local conflicts with --global" {
    run hk install --force-local --global
    assert_failure
    assert_output --partial "cannot be used with"
}
