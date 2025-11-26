#!/usr/bin/env mise run test:bats

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

# This test specifically checks for the newline stripping bug in origin/main
assert_exact_newline_preservation() {
    run bash -lc 'python3 - <<"PY"
import pathlib
import subprocess
import sys

worktree = pathlib.Path("newline_test.txt").read_bytes()
index = subprocess.check_output(["git", "show", ":newline_test.txt"])

# The index should have exactly "fixed\n\n" (5 bytes + 2 newlines = 7 bytes)
expected_index = b"fixed\n\n"
if index != expected_index:
    print(f"ERROR: Index content is wrong")
    print(f"  Expected (len={len(expected_index)}): {repr(expected_index)}")
    print(f"  Got (len={len(index)}): {repr(index)}")
    sys.exit(1)

# The worktree should have exactly "fixed\n\nExtra\n" (13 bytes)
expected_worktree = b"fixed\n\nExtra\n"
if worktree != expected_worktree:
    print(f"ERROR: Worktree content is wrong")
    print(f"  Expected (len={len(expected_worktree)}): {repr(expected_worktree)}")
    print(f"  Got (len={len(worktree)}): {repr(worktree)}")
    sys.exit(2)

print("SUCCESS: Exact newline preservation verified")
PY'
    assert_success
}

@test "fix preserves exact double newlines in middle of content" {
    # Test that double newlines in the middle of content are preserved
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["formatter"] {
        glob = "double.txt"
        stage = "double.txt"
        // Formatter that uppercases content
        fix = "tr '[:lower:]' '[:upper:]' < double.txt > double.tmp && mv double.tmp double.txt"
      }
    }
  }
}
PKL
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install

    # Create base file
    printf 'base\n' > double.txt
    git add double.txt
    git -c commit.gpgsign=false commit -m "base"

    # Stage content with double newline in middle
    printf 'line1\n\nline2\n' > double.txt
    git add double.txt

    # Worktree adds extra line at end
    printf 'line1\n\nline2\nline3\n' > double.txt

    run hk fix -v
    assert_success

    # Check exact bytes
    run bash -lc 'python3 - <<"PY"
import subprocess
import sys

worktree = open("double.txt", "rb").read()
index = subprocess.check_output(["git", "show", ":double.txt"])

# Index should have uppercased staged content
expected_index = b"LINE1\n\nLINE2\n"
if index != expected_index:
    print(f"ERROR: Index content wrong")
    print(f"  Expected: {repr(expected_index)}")
    print(f"  Got: {repr(index)}")
    sys.exit(1)

# Worktree should preserve the unstaged line3
expected_worktree = b"LINE1\n\nLINE2\nline3\n"  # line3 stays lowercase as it was unstaged
if worktree != expected_worktree:
    print(f"ERROR: Worktree content wrong")
    print(f"  Expected: {repr(expected_worktree)}")
    print(f"  Got: {repr(worktree)}")
    sys.exit(2)

print("SUCCESS: Double newlines preserved correctly")
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

@test "fix preserves exact trailing newlines when only unstaged has extra newlines" {
    # This test specifically targets the .read() bug that strips trailing newlines
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["formatter"] {
        glob = "newline_test.txt"
        stage = "newline_test.txt"
        // This formatter just adds "fixed" prefix and preserves rest
        fix = "printf 'fixed\\n\\n' > newline_test.txt"
      }
    }
  }
}
PKL
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install

    # Create base file
    printf 'base\n' > newline_test.txt
    git add newline_test.txt
    git -c commit.gpgsign=false commit -m "base"

    # Stage content with double newline at end
    printf 'content\n\n' > newline_test.txt
    git add newline_test.txt

    # Worktree has extra content after the double newline
    printf 'content\n\nExtra\n' > newline_test.txt

    # Run fix
    # BUG in origin/main: .read() strips trailing newline from index_pre
    # So it reads "content\n" instead of "content\n\n"
    # This causes incorrect tail detection: thinks "\nExtra\n" is the tail
    # Result: worktree gets corrupted with wrong merge
    run hk fix -v
    assert_success

    # Use exact newline preservation check
    assert_exact_newline_preservation
}

