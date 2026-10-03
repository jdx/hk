#!/usr/bin/env bats

# hk finds the stash entry it created by its commit id. The stash is shared by
# every worktree of a repository, so `stash@{n}` can name someone else's entry
# by the time hk restores.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Writes a pre-commit hook whose step runs $1 while the unstaged changes are
# stashed.
write_config() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["interfere"] {
        glob = "**/*.txt"
        fix = "$1"
      }
    }
  }
}
PKL
}

# A repository with a staged and an unstaged edit to file.txt, and a second
# worktree of it with an edit of its own.
prepare_repo() {
    printf 'base\n' > file.txt
    printf 'base\n' > other.txt
    git add hk.pkl file.txt other.txt
    git commit -m "init"
    git worktree add -q "$TEST_TEMP_DIR/other-worktree" -b other
    printf 'other worktree edit\n' > "$TEST_TEMP_DIR/other-worktree/other.txt"
    printf 'staged\n' > file.txt
    git add file.txt
    printf 'staged\nunstaged\n' > file.txt
}

assert_foreign_push_leaves_hk_entry_alone() {
    local use_libgit2="$1"
    write_config "git -C '$TEST_TEMP_DIR/other-worktree' stash push -m foreign-entry"
    prepare_repo

    run env HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_success

    # hk's unstaged edit is back, the staged one is still staged
    assert_equal "$(cat file.txt)" "$(printf 'staged\nunstaged\n')"
    assert_equal "$(git show :file.txt)" "staged"
    # hk dropped its own entry and only that one
    run git stash list
    assert_success
    assert_line --partial "foreign-entry"
    assert_equal "$(git stash list | wc -l | tr -d ' ')" "1"
    # the foreign entry still holds the other worktree's edit
    assert_equal "$(git show 'stash@{0}:other.txt')" "other worktree edit"
    assert_equal "$(cat "$TEST_TEMP_DIR/other-worktree/other.txt")" "base"
}

@test "foreign stash push between stash and restore is untouched (libgit2)" {
    assert_foreign_push_leaves_hk_entry_alone 1
}

@test "foreign stash push between stash and restore is untouched (shell git)" {
    assert_foreign_push_leaves_hk_entry_alone 0
}

assert_missing_entry_is_reported_and_others_survive() {
    local use_libgit2="$1"
    # Something else drops hk's entry (the top one) and pushes its own
    write_config "git -C '$TEST_TEMP_DIR/other-worktree' stash drop --quiet && git -C '$TEST_TEMP_DIR/other-worktree' stash push -m foreign-entry"
    prepare_repo
    # An older entry that must also survive
    printf 'older\n' > "$TEST_TEMP_DIR/other-worktree/other.txt"
    git -C "$TEST_TEMP_DIR/other-worktree" stash push -q -m older-entry
    printf 'other worktree edit\n' > "$TEST_TEMP_DIR/other-worktree/other.txt"

    run env HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_failure
    assert_output --partial "is no longer in the stash list"
    assert_output --partial "left every stash entry alone"

    run git stash list
    assert_line --partial "foreign-entry"
    assert_line --partial "older-entry"
    assert_equal "$(git stash list | wc -l | tr -d ' ')" "2"
}

@test "an entry that vanished from the stash list is reported, not replaced (libgit2)" {
    assert_missing_entry_is_reported_and_others_survive 1
}

@test "an entry that vanished from the stash list is reported, not replaced (shell git)" {
    assert_missing_entry_is_reported_and_others_survive 0
}
