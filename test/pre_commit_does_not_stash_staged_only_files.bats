#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_SUMMARY_TEXT=1
}

teardown() {
    _common_teardown
}

# Minimal pre-commit that runs a no-op on *.txt to exercise stash path
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
        stage = "*.txt"
        fix = "bash -lc 'true'"
      }
    }
  }
}
PKL
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install
}

# Prepare a file with ONLY staged changes; no unstaged hunks
prepare_only_staged_change() {
    printf 'base\n' > a.txt
    git add a.txt
    git -c commit.gpgsign=false commit -m "base"

    printf 'staged\n' > a.txt
    git add a.txt

    # Sanity: there must be no unstaged diffs
    run bash -lc "git diff -- a.txt"
    assert_success
    assert_output ""
}

@test "pre-commit does not stash when there are no unstaged hunks" {
    create_minimal_precommit
    prepare_only_staged_change

    # Run hook and capture verbose output
    run bash -lc 'HK_LOG=debug hk run pre-commit || true'
    echo "$output"

    # Expectation: hk should detect no unstaged changes and avoid stashing
    # Current buggy behavior stashes anyway; assert the correct behavior so this fails now.
    # Use status as ground truth: no worktree changes should have been introduced either
    run bash -lc 'git status --porcelain --untracked-files=all'
    assert_success
    assert_output --partial "M  a.txt"
    refute_output --partial " M a.txt"
}

