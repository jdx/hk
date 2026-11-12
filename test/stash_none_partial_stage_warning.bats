#!/usr/bin/env bats

# Without stashing, a fixer runs on the working tree, so staging its fix also
# stages hunks the user left unstaged. hk warns instead of widening the commit
# silently.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

_partially_staged_fix() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "$1"
    steps {
      ["trim"] {
        glob = "*.txt"
        fix = #"sed -i.bak 's/[[:space:]]*$//' {{files}} && rm -f {{files}}.bak"#
      }
    }
  }
}
PKL
    printf 'a\nb\nc\nd\ne\n' > notes.txt
    git add hk.pkl notes.txt
    git -c commit.gpgsign=false commit -m init
    printf 'A  \nb\nc\nd\ne\n' > notes.txt
    git add notes.txt
    printf 'A  \nb\nc\nd\nE\n' > notes.txt
}

@test "pre-commit (stash=none) warns when a fixer stages unstaged hunks" {
    _partially_staged_fix none
    run hk run pre-commit
    assert_success
    assert_output --partial "staged notes.txt with the unstaged changes it had before the hook ran"
    run git diff --cached
    assert_output --partial "+E"
}

@test "pre-commit (stash=git) does not warn" {
    _partially_staged_fix git
    run hk run pre-commit
    assert_success
    refute_output --partial "unstaged changes it had before the hook ran"
    run git diff --cached
    refute_output --partial "+E"
}
