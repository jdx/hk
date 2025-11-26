#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

create_config_with_stash_method() {
    local method="$1"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "$method"
    steps = new Mapping<String, Step> {
      ["overwrite"] {
        glob = "conflict.txt"
        stage = "conflict.txt"
        fix = "printf 'fixed\\n' > conflict.txt"
      }
    }
  }
  ["fix"] {
    fix = true
    stash = "$method"
    steps = new Mapping<String, Step> {
      ["overwrite"] {
        glob = "conflict.txt"
        stage = "conflict.txt"
        fix = "printf 'fixed\\n' > conflict.txt"
      }
    }
  }
}
PKL
    git add hk.pkl
    git commit -m 'init'
    hk install
}

prepare_repo_with_conflicting_changes_and_untracked() {
    mkdir -p untracked_dir
    # Commit base
    printf 'base\n' > conflict.txt
    git add conflict.txt
    git commit -m "base"
    # Stage a change to same line
    printf 'staged\n' > conflict.txt
    git add conflict.txt
    # Make an unstaged change to same line
    printf 'unstaged\n' > conflict.txt
    # Add unrelated untracked
    printf 'u\n' > untracked_dir/file.txt
}

assert_head_contains() {
    local needle="$1"
    run bash -c "git show HEAD:conflict.txt | grep -F '$needle'"
    assert_success
}

assert_head_not_contains() {
    local needle="$1"
    run bash -c "git show HEAD:conflict.txt | grep -F '$needle'"
    assert_failure
}

@test "stash=git: worktree keeps unstaged; no conflicts" {
    create_config_with_stash_method git
    prepare_repo_with_conflicting_changes_and_untracked
    run hk fix -v
    assert_success
    # Worktree should preserve unstaged changes without conflicts; fixer output not in worktree
    run grep -q 'unstaged' conflict.txt
    assert_success
    run grep -q 'fixed' conflict.txt
    assert_failure
    run grep -q '<<<<<<<' conflict.txt
    assert_failure
    run grep -q '>>>>>>>' conflict.txt
    assert_failure
}

@test "stash=patch-file: worktree keeps unstaged; untracked preserved; no conflicts" {
    create_config_with_stash_method patch-file
    prepare_repo_with_conflicting_changes_and_untracked
    run hk fix -v
    assert_success
    # Worktree should preserve unstaged changes without conflicts; fixer output not in worktree
    run grep -q 'unstaged' conflict.txt
    assert_success
    run grep -q 'fixed' conflict.txt
    assert_failure
    run grep -q '<<<<<<<' conflict.txt
    assert_failure
    run grep -q '>>>>>>>' conflict.txt
    assert_failure
    # Untracked files should remain
    run test -e untracked_dir/file.txt
    assert_success
}

