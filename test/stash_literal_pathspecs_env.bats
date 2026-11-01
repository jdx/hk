#!/usr/bin/env bats

# `git --literal-pathspecs commit` sets GIT_LITERAL_PATHSPECS=1 for the
# pre-commit hook, as tools that pass the flag to every git call do. hk writes
# its own pathspec magic (`:(literal)PATH`), which git then read as part of a
# file name: restoring a stashed untracked file and staging the fixed files
# matched nothing, and the commit failed with the stash left behind.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps { ["trailing-whitespace"] = Builtins.trailing_whitespace }
  }
}
EOF
    mkdir -p src
    echo a > src/a.txt
    git add hk.pkl src/a.txt
    git commit -qm init
    hk install
}

teardown() {
    _common_teardown
}

@test "pre-commit stashes and restores under GIT_LITERAL_PATHSPECS" {
    echo 'fixed  ' > src/a.txt
    echo untracked > src/new.rs

    run git --literal-pathspecs commit --only -m change -- src/a.txt
    assert_success
    # The step's fix is in the commit, the untracked file is back, and no
    # stash is left over.
    run git show HEAD:src/a.txt
    assert_output 'fixed'
    run cat src/new.rs
    assert_output 'untracked'
    run git for-each-ref refs/hk/stash
    assert_output ''
}
