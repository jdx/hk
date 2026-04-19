#!/usr/bin/env bats

# Regression tests for #831 — support bare-repo dotfile managers (YADM, etc.)
# that set GIT_DIR and GIT_WORK_TREE instead of using a `.git` in the worktree.

setup() {
    load 'test_helper/common_setup'
    _common_setup

    BARE_DIR="$TEST_TEMP_DIR/bare.git"
    WORK_TREE="$TEST_TEMP_DIR/home"
    git init --bare "$BARE_DIR"
    mkdir -p "$WORK_TREE"

    export GIT_DIR="$BARE_DIR"
    export GIT_WORK_TREE="$WORK_TREE"
    cd "$WORK_TREE"

    echo "initial" > file.txt
    git add file.txt
    git commit -m "initial commit"
}

teardown() {
    unset GIT_DIR
    unset GIT_WORK_TREE
    _common_teardown
}

_write_hk_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["echo"] { check = "echo checked {{files}}" } } }
    ["pre-commit"] { steps { ["echo"] { check = "echo pre-commit {{files}}" } } }
}
EOF
}

@test "hk builtins works with no repo config" {
    # Outside any repo, with no hk.pkl, should not panic
    cd "$TEST_TEMP_DIR"
    unset GIT_DIR
    unset GIT_WORK_TREE
    run hk builtins
    assert_success
    assert_output --partial "prettier"
}

@test "hk check honors GIT_DIR/GIT_WORK_TREE" {
    _write_hk_config
    git add hk.pkl
    git commit -m "add hk config"

    run hk check --all
    assert_success
    assert_output --partial "checked"
}

@test "hk check with HK_LIBGIT2=0 honors GIT_DIR/GIT_WORK_TREE" {
    _write_hk_config
    git add hk.pkl
    git commit -m "add hk config"

    HK_LIBGIT2=0 run hk check --all
    assert_success
    assert_output --partial "checked"
}

@test "hk install writes hooks to the bare-repo hooks dir" {
    _write_hk_config

    run hk install
    assert_success
    assert_file_exists "$BARE_DIR/hooks/pre-commit"
}

@test "hk uninstall removes hooks from the bare-repo hooks dir" {
    _write_hk_config

    hk install
    assert_file_exists "$BARE_DIR/hooks/pre-commit"

    run hk uninstall
    assert_success
    assert_file_not_exists "$BARE_DIR/hooks/pre-commit"
}

@test "hk check picks up modified files when run from a subdirectory" {
    # Regression for the reviewer-flagged bug: when GIT_DIR/GIT_WORK_TREE is
    # set and cwd is a subdirectory of the work tree, Git::new() must cd to
    # the work-tree root so path.exists() checks in status() resolve against
    # the right directory. Without this, modified files silently disappear
    # from the file list.
    _write_hk_config
    mkdir -p sub
    echo "original" > top.txt
    git add hk.pkl top.txt
    git commit -m "add tracked file"

    # Modify the file at the work tree root, then run hk check from a subdir.
    echo "modified" > top.txt

    cd sub
    run hk check
    assert_success
    assert_output --partial "checked"
    assert_output --partial "top.txt"
}
