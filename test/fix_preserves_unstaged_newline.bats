#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

create_fix_config_with_stash() {
    local stash_method="$1"
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    stash = "$stash_method"
    steps = new Mapping<String, Step> {
      ["formatter"] {
        glob = "sample.txt"
        stage = "sample.txt"
        fix = "printf 'formatted\\n' > sample.txt"
      }
    }
  }
}
PKL
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install
}

prepare_sample_file() {
    printf 'base\n' > sample.txt
    git add sample.txt
    git -c commit.gpgsign=false commit -m "base"

    printf 'A\nB\n' > sample.txt
    git add sample.txt

    cat <<'EOF' > sample.txt
A
B
C
EOF
}

assert_preserves_newline() {
    run bash -lc 'python3 - <<"PY"
import pathlib
import subprocess
import sys

worktree = pathlib.Path("sample.txt").read_bytes()
if not worktree.endswith(b"\n"):
    sys.exit(1)

index = subprocess.check_output(["git", "show", ":sample.txt"])
if not index.endswith(b"\n"):
    sys.exit(2)

# ensure contents match exactly between worktree and index, including newline
if worktree != index:
    sys.exit(3)
PY'
    assert_success
}

@test "fix (stash=git) preserves trailing newline in worktree and staged file" {
    create_fix_config_with_stash git
    prepare_sample_file

    run hk fix -v
    assert_success

    assert_preserves_newline

    # Check that the staged file has been formatted (not HEAD which hasn't been committed)
    run bash -lc "git show :sample.txt"
    assert_success
    assert_output --partial "formatted"
}

@test "fix (stash=patch-file) preserves trailing newline in worktree and staged file" {
    create_fix_config_with_stash patch-file
    prepare_sample_file

    run hk fix -v
    assert_success

    assert_preserves_newline

    # Check that the staged file has been formatted (not HEAD which hasn't been committed)
    run bash -lc "git show :sample.txt"
    assert_success
    assert_output --partial "formatted"
}
