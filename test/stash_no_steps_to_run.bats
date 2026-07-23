#!/usr/bin/env bats

# Regression test for https://github.com/jdx/hk/discussions/1105
#
# When a hook resolves to zero steps but the worktree has unstaged/untracked
# changes, hk used to stash those changes and then return early *before*
# restoring the stash, leaving the working tree stripped and a dangling stash.
# It should not stash at all when there is nothing to run.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_STASH_UNTRACKED=true
}

teardown() {
    _common_teardown
}

@test "no stash created when hook has no steps to run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {}
  }
}
PKL
    git add hk.pkl
    git commit -m 'init'

    # A staged change, an unstaged change, and an untracked file
    echo 'original' > tracked.txt
    git add tracked.txt
    git commit -m 'add tracked'

    echo 'staged change' > tracked.txt
    git add tracked.txt
    echo 'unstaged change' > tracked.txt
    echo 'untracked content' > untracked.txt

    stash_count_before=$(git stash list | wc -l)

    run hk run pre-commit
    assert_success
    assert_output --partial "no steps to run"
    refute_output --partial "Stashed unstaged changes"

    # No stash should have been left behind
    stash_count_after=$(git stash list | wc -l)
    assert_equal "$stash_count_before" "$stash_count_after"

    # Working tree must be intact
    run test -f untracked.txt
    assert_success
    run cat untracked.txt
    assert_output 'untracked content'
    run cat tracked.txt
    assert_output 'unstaged change'
}

@test "no steps to run leaves worktree intact through git commit" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {}
  }
  ["commit-msg"] {
    steps {
      ["always-fail"] { check = "false" }
    }
  }
}
PKL
    git add hk.pkl
    git commit -m 'init'

    echo 'a' > a.md
    git add a.md
    git commit -m 'add a'

    # Install hooks only after setup so the always-fail commit-msg does not
    # block the setup commits above.
    hk install

    git mv a.md b.md          # staged rename
    echo 'untracked' > u.md   # untracked file

    stash_count_before=$(git stash list | wc -l)

    # commit-msg fails on purpose, aborting the commit
    run git commit -m 'test'
    assert_failure

    # Even though the commit aborted, the pre-commit hook must not have left a
    # dangling stash or stripped the working tree.
    stash_count_after=$(git stash list | wc -l)
    assert_equal "$stash_count_before" "$stash_count_after"

    run test -f u.md
    assert_success
    run test -f b.md
    assert_success
    run cat u.md
    assert_output 'untracked'
}
