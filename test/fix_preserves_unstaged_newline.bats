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
index = subprocess.check_output(["git", "show", ":sample.txt"])

# Check that formatted content is in the index
if not index.startswith(b"formatted\n"):
    print(f"Index doesn'\''t start with formatted: {index}")
    sys.exit(1)

# Check that worktree preserves the unstaged tail
if not worktree.startswith(b"formatted\n"):
    print(f"Worktree doesn'\''t start with formatted: {worktree}")
    sys.exit(2)

# Check that worktree has the tail appended
if not worktree.endswith(b"C\n"):
    print(f"Worktree doesn'\''t end with C newline: {worktree}")
    sys.exit(3)

print("SUCCESS: Index has formatted content, worktree preserves unstaged tail")
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
