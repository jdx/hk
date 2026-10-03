#!/usr/bin/env bats

# `git stash push` resets the worktree and with it ends an operation in
# progress: it deletes MERGE_HEAD, MERGE_MSG, MERGE_MODE, CHERRY_PICK_HEAD,
# REVERT_HEAD, SQUASH_MSG and AUTO_MERGE. A pre-commit hook that stashes must
# leave them, or the commit that finishes a merge, cherry-pick or revert is
# not the one git was preparing.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_SUMMARY_TEXT=1
    # An author from the environment would hide the one a cherry-pick records
    unset GIT_AUTHOR_NAME GIT_AUTHOR_EMAIL GIT_COMMITTER_NAME GIT_COMMITTER_EMAIL
    git config commit.gpgsign false
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps { ["noop"] { check = "[ x\$HK_TEST_FAIL != x1 ]" } }
  }
}
PKL
    printf 'one\n' > f.txt
    printf 'one\n' > g.txt
    printf 'one\n' > h.txt
    git add -A
    git commit -m init
    git branch -M main
    hk install
}

teardown() {
    _common_teardown
}

# A merge of `other` into main that waits for its commit.
start_merge() {
    git checkout -q -b other
    printf 'other\n' > g.txt
    git commit -qam "change g"
    git checkout -q main
    printf 'main\n' > h.txt
    git commit -qam "change h"
    git merge --no-commit --no-ff other
}

# A cherry-pick of a commit by someone else, with a conflict that is resolved
# and staged, which leaves the pick waiting for its commit.
start_cherry_pick() {
    git checkout -q -b other
    printf 'other\n' > g.txt
    git -c user.name='Other Author' -c user.email=other@example.com commit -qam "change g"
    git checkout -q main
    printf 'main\n' > g.txt
    git commit -qam "main g"
    run git cherry-pick other
    assert_failure
    printf 'resolved\n' > g.txt
    git add g.txt
}

merge_in_progress() {
    start_merge
    printf 'unstaged\n' > f.txt
    # Untracked files make the stash cover the whole worktree, not paths
    printf 'untracked\n' > untracked.txt

    HK_LIBGIT2=$1 run git commit --no-edit
    assert_success
    # The commit and its two parents (awk, because BSD wc pads its count)
    assert_equal "$(git rev-list --parents -n1 HEAD | awk '{print NF}')" 3
    assert_equal "$(git log -1 --format=%s)" "Merge branch 'other'"
    assert_equal "$(cat f.txt)" unstaged
    assert_equal "$(git stash list)" ""
}

@test "pre-commit keeps a merge in progress (HK_LIBGIT2=1)" {
    merge_in_progress 1
}

@test "pre-commit keeps a merge in progress (HK_LIBGIT2=0)" {
    merge_in_progress 0
}

cherry_pick_in_progress() {
    start_cherry_pick
    printf 'unstaged\n' > f.txt
    # Untracked files make the stash cover the whole worktree, not paths
    printf 'untracked\n' > untracked.txt

    HK_LIBGIT2=$1 run git commit --no-edit
    assert_success
    assert_equal "$(git log -1 --format=%an)" "Other Author"
    assert_equal "$(git log -1 --format=%s)" "change g"
    assert_equal "$(cat f.txt)" unstaged
    assert_equal "$(git stash list)" ""
}

@test "pre-commit keeps the author of a cherry-pick in progress (HK_LIBGIT2=1)" {
    cherry_pick_in_progress 1
}

@test "pre-commit keeps the author of a cherry-pick in progress (HK_LIBGIT2=0)" {
    cherry_pick_in_progress 0
}

merge_in_progress_with_intent_to_add() {
    start_merge
    printf 'unstaged\n' > f.txt
    # Untracked files make the stash cover the whole worktree, not paths
    printf 'untracked\n' > untracked.txt
    printf 'new\n' > new.txt
    git add -N new.txt

    HK_LIBGIT2=$1 run git commit --no-edit
    assert_success
    # The commit and its two parents (awk, because BSD wc pads its count)
    assert_equal "$(git rev-list --parents -n1 HEAD | awk '{print NF}')" 3
    assert_equal "$(cat f.txt)" unstaged
    assert_equal "$(git stash list)" ""
}

@test "pre-commit keeps a merge in progress when a file is intent-to-add (HK_LIBGIT2=1)" {
    merge_in_progress_with_intent_to_add 1
}

@test "pre-commit keeps a merge in progress when a file is intent-to-add (HK_LIBGIT2=0)" {
    merge_in_progress_with_intent_to_add 0
}

failing_step_keeps_merge() {
    start_merge
    printf 'unstaged\n' > f.txt
    # Untracked files make the stash cover the whole worktree, not paths
    printf 'untracked\n' > untracked.txt

    HK_TEST_FAIL=1 HK_LIBGIT2=$1 run git commit --no-edit
    assert_failure
    test -f "$(git rev-parse --git-path MERGE_HEAD)"
    test -f "$(git rev-parse --git-path MERGE_MSG)"
    assert_equal "$(cat f.txt)" unstaged
    assert_equal "$(git stash list)" ""
}

@test "pre-commit leaves a merge in progress when a step fails (HK_LIBGIT2=1)" {
    failing_step_keeps_merge 1
}

@test "pre-commit leaves a merge in progress when a step fails (HK_LIBGIT2=0)" {
    failing_step_keeps_merge 0
}

# A cherry-pick of two commits whose first conflicts. Committing the resolved
# first one leaves the second in the sequencer's todo list.
multi_commit_cherry_pick() {
    git checkout -q -b other
    printf 'other\n' > g.txt
    git -c user.name='Other Author' -c user.email=other@example.com commit -qam "change g"
    printf 'other\n' > h.txt
    git -c user.name='Other Author' -c user.email=other@example.com commit -qam "change h"
    git checkout -q main
    printf 'main\n' > g.txt
    git commit -qam "main g"
    run git cherry-pick other~1 other
    assert_failure
    printf 'resolved\n' > g.txt
    git add g.txt
    printf 'unstaged\n' > f.txt
    # Untracked files make the stash cover the whole worktree, not paths
    printf 'untracked\n' > untracked.txt

    HK_LIBGIT2=$1 run git commit --no-edit
    assert_success
    assert_equal "$(git log -1 --format=%s)" "change g"
    # The sequencer must still have the second commit
    run git cherry-pick --continue
    assert_success
    assert_equal "$(git log -1 --format=%s)" "change h"
    assert_equal "$(git log -1 --format=%an)" "Other Author"
    assert_equal "$(cat f.txt)" unstaged
    assert_equal "$(git stash list)" ""
}

@test "pre-commit keeps the sequencer of a multi-commit cherry-pick (HK_LIBGIT2=1)" {
    multi_commit_cherry_pick 1
}

@test "pre-commit keeps the sequencer of a multi-commit cherry-pick (HK_LIBGIT2=0)" {
    multi_commit_cherry_pick 0
}

# Stashing would delete a state file hk cannot read, so hk stops before it.
unreadable_state_file() {
    if [ "$(id -u)" = 0 ]; then
        skip "root reads every file"
    fi
    start_merge
    printf 'unstaged\n' > f.txt
    printf 'untracked\n' > untracked.txt
    local state
    state=$(git rev-parse --git-path MERGE_MSG)
    chmod 000 "$state"

    HK_LIBGIT2=$1 run hk run pre-commit
    chmod 644 "$state"
    assert_failure
    assert_output --partial "failed to read"
    assert_equal "$(git stash list)" ""
    assert_equal "$(cat f.txt)" unstaged
    test -f "$(git rev-parse --git-path MERGE_HEAD)"
}

@test "pre-commit stops when a merge state file cannot be read (HK_LIBGIT2=1)" {
    unreadable_state_file 1
}

@test "pre-commit stops when a merge state file cannot be read (HK_LIBGIT2=0)" {
    unreadable_state_file 0
}

# When the conflicting pick is the last one, committing it ends the cherry-pick.
# Keeping the sequencer must not leave one in progress afterwards.
last_commit_cherry_pick() {
    git checkout -q -b other
    printf 'other\n' > h.txt
    git -c user.name='Other Author' -c user.email=other@example.com commit -qam "change h"
    printf 'other\n' > g.txt
    git -c user.name='Other Author' -c user.email=other@example.com commit -qam "change g"
    git checkout -q main
    printf 'main\n' > g.txt
    git commit -qam "main g"
    run git cherry-pick other~1 other
    assert_failure
    printf 'resolved\n' > g.txt
    git add g.txt
    printf 'unstaged\n' > f.txt
    printf 'untracked\n' > untracked.txt

    HK_LIBGIT2=$1 run git commit --no-edit
    assert_success
    assert_equal "$(git log -1 --format=%s)" "change g"
    assert_equal "$(git log -1 --format=%an)" "Other Author"
    # Nothing is left in progress
    run git cherry-pick --continue
    assert_failure
    test ! -e "$(git rev-parse --git-path sequencer)"
    assert_equal "$(cat f.txt)" unstaged
}

@test "pre-commit leaves no cherry-pick in progress after the last pick (HK_LIBGIT2=1)" {
    last_commit_cherry_pick 1
}

@test "pre-commit leaves no cherry-pick in progress after the last pick (HK_LIBGIT2=0)" {
    last_commit_cherry_pick 0
}
