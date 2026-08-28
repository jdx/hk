#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    export HK_SUMMARY_TEXT=1
}

teardown() {
    _common_teardown
}

create_precommit() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stage = true
    stash = "git"
    steps {
      ["noop"] { glob = List("**/*.sh"); check = "true" }
    }
  }
}
PKL
    printf 'echo hi\n' > a.sh
    git add -A
    git -c commit.gpgsign=false commit -m init
    hk install
}

@test "pre-commit preserves a staged mode change when the stash patch is empty" {
    create_precommit

    git update-index --chmod=+x a.sh
    run git -c commit.gpgsign=false commit -m "make a.sh executable"
    assert_success

    run git ls-tree HEAD -- a.sh
    assert_success
    assert_output --partial "100755"

    run git stash list
    assert_success
    assert_output ""
}

@test "pre-commit preserves staged content when the worktree matches HEAD" {
    create_precommit

    printf 'echo staged\n' > a.sh
    git add a.sh
    printf 'echo hi\n' > a.sh

    run git -c commit.gpgsign=false commit -m "change a.sh"
    assert_success

    run git show HEAD:a.sh
    assert_success
    assert_output "echo staged"

    run cat a.sh
    assert_success
    assert_output "echo hi"

    run git stash list
    assert_success
    assert_output ""
}
