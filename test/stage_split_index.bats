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

# Writes a pre-commit hook whose fixer stages with `$1`, then stages a broken
# a.txt in a split index.
setup_split_index_fixer() {
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
        $1
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
}

assert_fix_staged() {
  run hk run pre-commit
  assert_success
  run git show :a.txt
  assert_output 'fixed'
  run git status --porcelain -- a.txt
  assert_output 'A  a.txt'
}

# The default `stage` reads the status of the step's files by path.
@test "pre-commit stages its fixes in a repository with a split index" {
  setup_split_index_fixer ""
  assert_fix_staged
}

# An explicit `stage` pattern reads the status by pathspec instead.
@test "pre-commit stages an explicit stage glob in a repository with a split index" {
  setup_split_index_fixer 'stage = List("*.txt")'
  assert_fix_staged
}
