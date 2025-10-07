#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_STASH_UNTRACKED=true
}

teardown() {
    _common_teardown
}

@test "partial stash should include untracked files when HK_STASH_UNTRACKED=true" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["test-step"] {
        glob = "tracked.txt"
        stage = "tracked.txt"
        fix = "echo 'modified' > tracked.txt"
      }
    }
  }
}
PKL
    git add hk.pkl
    git commit -m 'init'
    hk install

    # Create a tracked file with staged changes
    echo 'original' > tracked.txt
    git add tracked.txt
    git commit -m 'add tracked file'

    # Make changes to tracked file and stage them
    echo 'staged change' > tracked.txt
    git add tracked.txt

    # Create an untracked file that should be stashed away
    echo 'untracked content' > untracked_should_be_stashed.txt

    # Verify untracked file exists before commit
    run test -f untracked_should_be_stashed.txt
    assert_success

    # Run pre-commit hook - should stash untracked file
    run git commit -m 'test commit'
    assert_success

    # The untracked file should NOT have been added to the commit
    run bash -c "git show HEAD --name-only | grep untracked_should_be_stashed.txt"
    assert_failure

    # After commit, untracked file should still exist (restored from stash)
    run test -f untracked_should_be_stashed.txt
    assert_success

    # Verify untracked file has original content
    run cat untracked_should_be_stashed.txt
    assert_output 'untracked content'
}

@test "untracked files not staged by broad glob when they should have been stashed" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["test-formatter"] {
        glob = "*.txt"
        stage = "**/*"
        fix = "cat"
      }
    }
  }
}
PKL
    git add hk.pkl
    git commit -m 'init'
    hk install

    # Create a tracked file with changes
    echo 'tracked' > tracked.txt
    git add tracked.txt
    git commit -m 'add tracked'

    # Modify the tracked file
    echo 'modified' > tracked.txt
    git add tracked.txt

    # Create an untracked file that matches the stage glob
    echo 'untracked' > untracked.txt

    # Verify untracked file exists before commit
    run test -f untracked.txt
    assert_success

    # Run pre-commit hook
    run git commit -m 'test commit'
    assert_success

    # Check what was committed - untracked.txt should NOT be in the commit
    run bash -c "git show HEAD --name-only | grep untracked.txt"
    assert_failure

    # Untracked file should still exist in worktree (restored from stash)
    run test -f untracked.txt
    assert_success
}
