#!/usr/bin/env bats

# libgit2 cannot read a split index (`core.splitIndex`), so with libgit2 hk
# reads the status of the files it stages with git instead.

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

@test "pre-commit stages its fixes in a repository with a split index" {
  git config core.splitIndex true
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    steps {
      ["fix"] {
        glob = "*.txt"
        fix = "echo fixed > {{files}}"
      }
    }
  }
}
PKL
  git add hk.pkl
  git commit -qm "init hk"
  printf 'broken\n' > a.txt
  git add a.txt
  git update-index --split-index
  run ls .git
  assert_output --partial sharedindex.

  run hk run pre-commit
  assert_success
  run git show :a.txt
  assert_output 'fixed'
  run git status --porcelain -- a.txt
  assert_output 'A  a.txt'
}
