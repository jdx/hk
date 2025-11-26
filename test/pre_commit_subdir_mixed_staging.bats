#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Scenario: run git commit from a nested subdirectory. Ensure hk scoping keeps
# unrelated files outside of the step's job set from being included, even when
# there is a mixture of staged/unstaged files in cwd, child dirs, and sibling dirs.

create_precommit_prettier() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["prettier"] = Builtins.prettier
    }
  }
}
EOF
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install
}

prepare_repo_tree_with_mixed_states() {
    mkdir -p a/b c d/e

    # Base files across different directories
    printf 'export function f(){return 0}' > a/x.ts
    printf 'export function g(){return 0}' > a/b/y.ts
    printf 'export function h(){return 0}' > c/z.ts
    printf 'export function i(){return 0}' > d/e/w.ts
    git add a/x.ts a/b/y.ts c/z.ts d/e/w.ts
    git -c commit.gpgsign=false commit -m "base"

    # Stage change in cwd dir (a/), unstaged tail in same file
    printf 'export function f(){return 1}\n// u1' > a/x.ts
    blob=$(printf 'export function f(){return 1}' | git hash-object -w --stdin)
    git update-index --cacheinfo 100644 "$blob" a/x.ts

    # Stage change in child dir (a/b/), leave unstaged tail
    printf 'export function g(){return 2}\n// u2' > a/b/y.ts
    blob=$(printf 'export function g(){return 2}' | git hash-object -w --stdin)
    git update-index --cacheinfo 100644 "$blob" a/b/y.ts

    # Unrelated unstaged change in sibling dir (c/)
    printf 'export function h(){return 3}\n// u3' > c/z.ts
    git add -N c/z.ts  # intent-to-add style not needed but keep worktree change

    # Unrelated unstaged change in a different tree (d/e)
    printf 'export function i(){return 4}\n// u4' > d/e/w.ts

    # Sanity: staged should list only the two .ts under a/ and a/b/
    run bash -lc "git diff --staged --name-only | sort"
    assert_line 'a/b/y.ts'
    assert_line 'a/x.ts'
    refute_line 'c/z.ts'
    refute_line 'd/e/w.ts'
}

@test "git commit from subdir respects staging scope across directories" {
    create_precommit_prettier
    prepare_repo_tree_with_mixed_states

    # Change into subdirectory a/b and commit from there
    pushd a/b >/dev/null
    run git -c commit.gpgsign=false commit -m "subdir commit"
    popd >/dev/null
    assert_success

    # HEAD should include only staged files under a/ and a/b/, with Prettier formatting
    run bash -lc "git show --name-only --pretty=format: HEAD | sort"
    assert_line 'a/b/y.ts'
    assert_line 'a/x.ts'
    refute_line 'c/z.ts'
    refute_line 'd/e/w.ts'

    # Worktree should still contain the unstaged tails
    run bash -lc "grep -q '// u1' a/x.ts && echo ok || echo fail"
    assert_line 'ok'
    run bash -lc "grep -q '// u2' a/b/y.ts && echo ok || echo fail"
    assert_line 'ok'
    run bash -lc "grep -q '// u3' c/z.ts && echo ok || echo fail"
    assert_line 'ok'
    run bash -lc "grep -q '// u4' d/e/w.ts && echo ok || echo fail"
    assert_line 'ok'
}
