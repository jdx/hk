#!/usr/bin/env bats

# Regression test for https://github.com/jdx/hk/discussions/929
#
# When a fixer removes lines from the end of a file and the user has an
# unrelated unstaged change in the middle of the same file, the three-way
# merge inside pop_stash used to silently restore the deleted trailing lines.
# The bug was a missing branch in src/merge.rs::diff_hunks for pure tail
# deletions when the LCS walk consumed `other` entirely.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "pre-commit (stash=git) preserves fixer tail deletion when unstaged change is in middle" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["strip-trailing-blank-lines"] {
        glob = "*.yml"
        stage = "*.yml"
        // Remove all trailing blank lines, leaving exactly one final newline
        fix = #"perl -i -pe 'BEGIN{undef $/} s/\n+\z/\n/' {{files}}"#
      }
    }
  }
}
PKL
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install

    # Initial committed file (no trailing blank lines)
    cat <<'EOF' > config.yml
name: my-app
version: 1.0
deps:
  flask: 2.0
EOF
    git add config.yml
    git -c commit.gpgsign=false commit -m "base"

    # Stage a change that adds trailing blank lines (fixer will remove these)
    cat <<'EOF' > config.yml
name: my-app
version: 1.0
deps:
  flask: 2.0
  redis: 4.0


EOF
    git add config.yml

    # Add an unstaged change in the MIDDLE of the file (triggers stash flow)
    cat <<'EOF' > config.yml
name: my-app
version: 2.0
deps:
  flask: 2.0
  redis: 4.0


EOF

    run bash -lc 'git -c commit.gpgsign=false commit -m "redis"'
    echo "$output"
    assert_success

    # After the commit: fixer's tail deletion is preserved on disk and the
    # worktree retains the version=2.0 unstaged change. Compare with `diff`
    # (not bash command substitution, which strips trailing newlines and would
    # silently accept the very tail-blank-lines regression this test guards).
    expected=$'name: my-app\nversion: 2.0\ndeps:\n  flask: 2.0\n  redis: 4.0\n'
    printf '%s' "$expected" | diff - config.yml

    # The committed (HEAD) content is the fixer's output: redis added, no
    # trailing blank lines.
    expected_head=$'name: my-app\nversion: 1.0\ndeps:\n  flask: 2.0\n  redis: 4.0\n'
    git show HEAD:config.yml | diff - <(printf '%s' "$expected_head")
}
