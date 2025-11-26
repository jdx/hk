#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Guard step: during pre-commit, assert worktree does NOT contain pnpm-lock.yaml change.
# If it does, the step fails and the commit aborts, proving stash didn't run globally.

create_precommit_with_guard() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["guard-lockfile-stashed"] {
        glob = "*"
        // Fail if pnpm-lock.yaml currently differs from index/worktree baseline.
        // If stashed correctly, diff should be empty.
        check = "bash -lc 'git diff --name-only -- pnpm-lock.yaml | grep -q pnpm-lock.yaml && { echo not stashed; exit 1; } || exit 0'"
      }
      ["prettier"] = Builtins.prettier
    }
  }
}
EOF
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install
}

prepare_staged_ts_and_unstaged_lock() {
    printf 'export function f(){return 0}' > file.ts
    printf 'lock: base\n' > pnpm-lock.yaml
    git add file.ts pnpm-lock.yaml
    git -c commit.gpgsign=false commit -m "base"

    printf 'export function f(){return 2}' > file.ts
    blob=$(printf 'export function f(){return 2}' | git hash-object -w --stdin)
    git update-index --cacheinfo 100644 "$blob" file.ts

    # Unstaged lockfile change
    printf 'lock: base # foo\n' > pnpm-lock.yaml
}

@test "guard step fails if lockfile not stashed; passes when global stash works" {
    create_precommit_with_guard
    prepare_staged_ts_and_unstaged_lock

    # Run commit (pre-commit will run guard, which expects lockfile to be stashed)
    run git -c commit.gpgsign=false commit -m "test"
    assert_success

    # Confirm pnpm-lock.yaml was not committed
    run bash -lc "git show --name-only --pretty=format: HEAD"
    refute_line 'pnpm-lock.yaml'

    # Ensure no stash remains after pre-commit completes
    run bash -lc 'git stash list'
    assert_success
    assert_output ""
}

