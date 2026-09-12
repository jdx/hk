#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_SUMMARY_TEXT=1
}

teardown() {
    _common_teardown
}

# Minimal pre-commit with stash=git and a no-op step so the stash path runs
create_minimal_precommit() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["noop"] {
        glob = "*.txt"
        check = "bash -lc 'true'"
      }
    }
  }
}
PKL
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install
}

# Create a conflicting merge, resolve it, and stage the resolution.
# Leaves the repo mid-merge (MERGE_HEAD present).
prepare_resolved_merge() {
    printf 'base\n' > conflict.txt
    printf 'other\n' > other.txt
    git add conflict.txt other.txt
    git -c commit.gpgsign=false commit -m "base"
    git switch -c side
    printf 'side\n' > conflict.txt
    git -c commit.gpgsign=false commit -am "side"
    git switch main
    printf 'main\n' > conflict.txt
    git -c commit.gpgsign=false commit -am "main change"
    git switch side
    run git merge main
    assert_failure
    printf 'resolved\n' > conflict.txt
    git add conflict.txt
    assert_file_exists .git/MERGE_HEAD
}

@test "stash=git: skipped during merge; untracked-only dirt; merge commit keeps both parents" {
    create_minimal_precommit
    prepare_resolved_merge
    # Only an untracked file is dirty: without the merge guard this takes the
    # full-stash path (git stash push / libgit2 stash_save), whose internal
    # hard reset deletes MERGE_HEAD/MERGE_MSG and corrupts the merge commit.
    printf 'u\n' > untracked.tmp

    run git -c commit.gpgsign=false commit --no-edit
    assert_success

    # HEAD must be a real merge commit (two parents)
    run git rev-parse -q --verify HEAD^2
    assert_success
    # Untracked file untouched, no stray stash entries
    assert_file_exists untracked.tmp
    run git stash list
    assert_output ""
}

@test "stash=git: skipped during merge; unstaged tracked change preserved" {
    create_minimal_precommit
    prepare_resolved_merge
    # Unstaged change to a tracked file alongside the staged merge resolution
    printf 'unstaged\n' > other.txt

    run git -c commit.gpgsign=false commit --no-edit
    assert_success

    run git rev-parse -q --verify HEAD^2
    assert_success
    # Unstaged change still in the worktree, not committed
    run grep -q 'unstaged' other.txt
    assert_success
    run bash -c "git show HEAD:other.txt | grep -F 'unstaged'"
    assert_failure
    run git stash list
    assert_output ""
}
