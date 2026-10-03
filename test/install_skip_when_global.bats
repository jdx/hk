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

@test "hk install --force-local on top of a global install runs hk once and does not suggest disabling it" {
    if ! git version | awk '{split($3,v,"."); exit !(v[1]>2 || (v[1]==2 && v[2]>=54))}'; then
        skip "git 2.54+ required for config-based hooks"
    fi
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["count"] { check = "echo ran >> \$PWD/runs.log" } } } }
EOF
    git add hk.pkl

    hk install --global
    run hk install --force-local
    assert_success
    # The local hook shares the global hook's name, so `enabled false` would
    # silence the local hook too. The message must not recommend it.
    refute_output --partial "enabled false"
    assert_output --partial "local hook replaces the global one"
    # The uninstall advice says what it removes and how to get it back.
    assert_output --partial "removes every local hk hook"
    assert_output --partial "hk install --force-local"

    echo x > file.txt
    git add file.txt
    git commit -m one
    # Local and global hooks share one name: hk runs once, not twice.
    run wc -l < runs.log
    assert_output --regexp '^ *1$'
}

@test "hk install --force-local says so when the hook is disabled in git config" {
    if ! git version | awk '{split($3,v,"."); exit !(v[1]>2 || (v[1]==2 && v[2]>=54))}'; then
        skip "git 2.54+ required for config-based hooks"
    fi
    _write_hk_pkl
    hk install --global
    git config --global hook.hk-pre-commit.enabled false

    run hk install --force-local
    assert_success
    assert_output --partial "git runs no hk hook for: pre-commit"
    assert_output --partial "git config --local hook.hk-pre-commit.enabled true"
    refute_output --partial "hk runs once per event"
}
