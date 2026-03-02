#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "pre-commit with stash does not pass untracked files to steps" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["list-files"] {
        glob = "*.txt"
        fix = "echo '<JOB_FILES>'"
      }
    }
  }
}
PKL
    git add hk.pkl
    git commit -m 'init'
    hk install

    # Create a tracked file and commit it
    echo 'tracked' > tracked.txt
    git add tracked.txt
    git commit -m 'add tracked file'

    # Stage a modification to the tracked file
    echo 'modified' > tracked.txt
    git add tracked.txt

    # Create an untracked file that matches the glob
    echo 'untracked' > untracked.txt

    # Run pre-commit via hk run (not git commit, to see output)
    run hk run pre-commit
    assert_success

    # The step should process only 1 file (the staged tracked.txt), not 2
    assert_output --partial "1 file"
    refute_output --partial "2 files"
    refute_output --partial "untracked.txt"
}
