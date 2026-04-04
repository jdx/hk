#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup

    echo "init" > file.txt
    git add file.txt
    git commit -m "initial commit"

    # Enable per-worktree config and create a worktree
    git config extensions.worktreeConfig true
    WORKTREE_DIR="$TEST_TEMP_DIR/worktree"
    git worktree add "$WORKTREE_DIR" -b test-branch

    # Set per-worktree core.hooksPath for the worktree
    WORKTREE_GIT_DIR="$(cd "$WORKTREE_DIR" && git rev-parse --git-dir)"
    WORKTREE_HOOKS="$WORKTREE_GIT_DIR/hooks"
    cd "$WORKTREE_DIR"
    git config --worktree core.hooksPath "$WORKTREE_HOOKS"
}

teardown() {
    _common_teardown
}

_write_hk_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks { ["pre-commit"] { steps { ["echo"] { check = "echo ok" } } } }
EOF
}

@test "hk install writes hooks to per-worktree hooksPath" {
    _write_hk_config

    run hk install
    assert_success
    assert_file_exists "$WORKTREE_HOOKS/pre-commit"

    # The shared hooks dir should not have an hk hook
    common_hooks="$(git rev-parse --git-common-dir)/hooks"
    if [ -f "$common_hooks/pre-commit" ]; then
        run cat "$common_hooks/pre-commit"
        refute_output --partial "hk run"
    fi
}

@test "hk uninstall removes hooks from per-worktree hooksPath" {
    _write_hk_config

    hk install
    assert_file_exists "$WORKTREE_HOOKS/pre-commit"

    run hk uninstall
    assert_success
    assert_file_not_exists "$WORKTREE_HOOKS/pre-commit"
}

@test "pre-commit hook fires via per-worktree hooksPath" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["linter"] { check = "echo linter-ran" }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "add hk config"

    hk install

    echo "new content" > newfile.txt
    git add newfile.txt
    run git commit -m "test commit"
    assert_success
    assert_output --partial "linter-ran"
}

@test "hk install falls back to shared hooks without per-worktree hooksPath" {
    # Unset the per-worktree hooksPath set in setup
    git config --worktree --unset core.hooksPath
    _write_hk_config

    run hk install
    assert_success

    # Hooks should be in the shared dir, not the worktree-local dir
    common_hooks="$(git rev-parse --git-common-dir)/hooks"
    assert_file_exists "$common_hooks/pre-commit"
    assert_file_not_exists "$WORKTREE_HOOKS/pre-commit"
}

@test "different worktrees get independent hooks" {
    _write_hk_config
    hk install

    # Create and configure a second worktree
    WORKTREE_DIR2="$TEST_TEMP_DIR/worktree2"
    cd "$TEST_TEMP_DIR/src/proj"
    git worktree add "$WORKTREE_DIR2" -b test-branch-2
    WORKTREE2_GIT_DIR="$(cd "$WORKTREE_DIR2" && git rev-parse --git-dir)"
    WORKTREE2_HOOKS="$WORKTREE2_GIT_DIR/hooks"
    cd "$WORKTREE_DIR2"
    git config --worktree core.hooksPath "$WORKTREE2_HOOKS"
    _write_hk_config
    hk install

    assert_file_exists "$WORKTREE_HOOKS/pre-commit"
    assert_file_exists "$WORKTREE2_HOOKS/pre-commit"
    [ "$WORKTREE_HOOKS" != "$WORKTREE2_HOOKS" ]
}
