#!/usr/bin/env bats

# hk keeps the stash entry it creates under its own ref, `refs/hk/stash/<commit>`,
# and finds it again by its commit id. The `git stash` stack is shared by every
# worktree of a repository, so another worktree pushing, popping or dropping
# there while a hook runs must not reach hk's entry.

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

# Like write_config, but the step has no glob, so the whole worktree is stashed
write_config_whole() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["interfere"] {
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
    # hk's entry is gone, and the foreign one is on the stash untouched
    assert_equal "$(hk_stash_list)" ""
    run git stash list
    assert_success
    assert_line --partial "foreign-entry"
    assert_equal "$(git stash list | wc -l | tr -d ' ')" "1"
    assert_equal "$(git show 'stash@{0}:other.txt')" "other worktree edit"
    assert_equal "$(cat "$TEST_TEMP_DIR/other-worktree/other.txt")" "base"
}

@test "foreign stash push between stash and restore is untouched (named paths)" {
    assert_foreign_push_leaves_hk_entry_alone 1
}

@test "foreign stash push between stash and restore is untouched (shell git)" {
    assert_foreign_push_leaves_hk_entry_alone 0
}

@test "foreign stash push between stash and restore is untouched (whole worktree)" {
    write_config_whole "git -C '$TEST_TEMP_DIR/other-worktree' stash push -m foreign-entry"
    prepare_repo
    printf 'untracked\n' > untracked.txt
    run env HK_STASH_UNTRACKED=true hk run pre-commit
    assert_success
    assert_equal "$(cat untracked.txt)" "untracked"
    assert_equal "$(cat file.txt)" "$(printf 'staged\nunstaged\n')"
    assert_equal "$(git show :file.txt)" "staged"
    assert_equal "$(hk_stash_list)" ""
    assert_equal "$(git stash list | wc -l | tr -d ' ')" "1"
    assert_equal "$(git show 'stash@{0}:other.txt')" "other worktree edit"
}

# hk records nothing when nothing is stashed, and a foreign push meanwhile is
# not mistaken for hk's
@test "nothing to stash with a foreign push in between touches no entry" {
    write_config "git -C '$TEST_TEMP_DIR/other-worktree' stash push -m foreign-entry"
    printf 'base\n' > file.txt
    git add hk.pkl file.txt
    git commit -m "init"
    git worktree add -q "$TEST_TEMP_DIR/other-worktree" -b other
    printf 'x\n' > "$TEST_TEMP_DIR/other-worktree/file.txt"
    printf 'staged\n' > file.txt
    git add file.txt
    run hk run pre-commit
    assert_success
    assert_equal "$(git stash list | wc -l | tr -d ' ')" "1"
    run git stash list
    assert_line --partial "foreign-entry"
    assert_equal "$(hk_stash_list)" ""
}

# The race from jdx/hk#1746: another worktree pops, drops and pushes on the
# shared stack while hk's hook runs. hk's entry is not on that stack.
@test "another worktree dropping and pushing on the shared stash does not reach hk's entry" {
    write_config "git -C '$TEST_TEMP_DIR/other-worktree' stash drop --quiet && git -C '$TEST_TEMP_DIR/other-worktree' stash push -q -m foreign-entry"
    prepare_repo
    # Both entries are on the shared stack before the hook starts
    printf 'older\n' > "$TEST_TEMP_DIR/other-worktree/other.txt"
    git -C "$TEST_TEMP_DIR/other-worktree" stash push -q -m older-entry
    printf 'other worktree edit\n' > "$TEST_TEMP_DIR/other-worktree/other.txt"

    run hk run pre-commit
    assert_success

    assert_equal "$(cat file.txt)" "$(printf 'staged\nunstaged\n')"
    assert_equal "$(git show :file.txt)" "staged"
    assert_equal "$(hk_stash_list)" ""
    # The other worktree's own stash operations ran as it expected
    run git stash list
    assert_line --partial "foreign-entry"
    assert_equal "$(git stash list | wc -l | tr -d ' ')" "1"
}

@test "an entry whose ref was deleted is reported, and other entries survive" {
    write_config "git update-ref -d refs/hk/stash/\$(git for-each-ref --format='%(objectname)' refs/hk/stash | head -1)"
    prepare_repo
    printf 'older\n' > "$TEST_TEMP_DIR/other-worktree/other.txt"
    git -C "$TEST_TEMP_DIR/other-worktree" stash push -q -m older-entry
    printf 'other worktree edit\n' > "$TEST_TEMP_DIR/other-worktree/other.txt"

    run hk run pre-commit
    assert_failure
    assert_output --partial "is no longer in hk's stash entries"
    assert_output --partial "left every stash entry alone"

    run git stash list
    assert_line --partial "older-entry"
    assert_equal "$(git stash list | wc -l | tr -d ' ')" "1"
}
