#!/usr/bin/env bats

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
}
PKL
    git add hk.pkl
    git commit -m 'init'
    hk install
}

prepare_repo_with_conflicting_changes_and_untracked() {
    mkdir -p untracked_dir
    printf 'base\n' > conflict.txt
    git add conflict.txt
    printf 'unstaged\n' > conflict.txt
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

@test "stash=git: current behavior commits unstaged changes in pre-commit" {
    skip "documents previous behavior; kept for reference"
    create_config_with_stash_method git
    prepare_repo_with_conflicting_changes_and_untracked
    run git commit -m 'Add conflict.txt'
    assert_success
    # Current behavior: unstaged content ends up in the commit (buggy)
    assert_head_contains 'unstaged'
    assert_head_not_contains 'fixed'
    # And the working tree should not show conflict.txt as modified
    run bash -c "git status --porcelain | grep '^ M .*conflict.txt'"
    assert_failure
}

@test "stash=patch-file: current behavior commits unstaged changes and removes untracked" {
    skip "documents previous behavior; kept for reference"
    create_config_with_stash_method patch-file
    prepare_repo_with_conflicting_changes_and_untracked
    run git commit -m 'Add conflict.txt'
    assert_success
    # Current behavior: unstaged content ends up in the commit (buggy)
    assert_head_contains 'unstaged'
    assert_head_not_contains 'fixed'
    # Untracked files are removed under patch-file strategy currently
    run test -e untracked_dir/file.txt
    assert_failure
}
