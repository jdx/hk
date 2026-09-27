#!/usr/bin/env bats

# libgit2 cannot read a split index (`core.splitIndex`), so with libgit2 hk
# reads the status of the files it stages with git instead.

setup() {
  load 'test_helper/common_setup'
  _common_setup
  git config core.splitIndex true
}

teardown() {
  _common_teardown
}

# Commits hk.pkl, then stages a broken a.txt in a split index.
stage_broken_file_in_split_index() {
  git add hk.pkl
  git commit -qm "init hk"
  printf 'broken\n' > a.txt
  git add a.txt
  git update-index --split-index
  run ls .git
  assert_output --partial sharedindex.
}

assert_fix_staged() {
  run git show :a.txt
  assert_output 'fixed'
  run git status --porcelain -- a.txt
  assert_output 'A  a.txt'
}

# The default `stage` reads the status of the step's files by path.
@test "pre-commit stages its fixes in a repository with a split index" {
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
  stage_broken_file_in_split_index

  run hk run pre-commit
  assert_success
  assert_fix_staged
}

# An explicit `stage` pattern reads the status by pathspec instead.
@test "pre-commit stages an explicit stage glob in a repository with a split index" {
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    steps {
      ["fix"] {
        glob = "*.txt"
        stage = List("*.txt")
        fix = "echo fixed > {{files}}"
      }
    }
  }
}
PKL
  stage_broken_file_in_split_index

  run hk run pre-commit
  assert_success
  assert_fix_staged
}

# `git commit` hands the hook its index through GIT_INDEX_FILE.
@test "git commit runs the pre-commit hook in a repository with a split index" {
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
  stage_broken_file_in_split_index
  hk install

  # Not a login shell: macOS path_helper would put /usr/bin/git first, and an
  # older git ignores the config-based hook hk installed.
  run bash -c 'git -c commit.gpgsign=false commit -m "add a.txt"'
  assert_success
  run git show HEAD:a.txt
  assert_output 'fixed'
}
