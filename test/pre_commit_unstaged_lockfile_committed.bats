#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# This reproduces a regression where an unstaged lockfile change (e.g. pnpm-lock.yaml)
# is accidentally included in the commit after running hk pre-commit.
# We simulate a Prettier step that formats a staged .ts file while an unrelated lockfile
# has an unstaged edit. The commit must NOT include the lockfile change.

create_precommit_with_prettier() {
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

prepare_repo_with_staged_ts_and_unstaged_lockfile() {
    # Base files
    printf 'export function f(){return 1}' > file.ts
    printf 'name: test\n' > pnpm-lock.yaml
    git add file.ts pnpm-lock.yaml
    git -c commit.gpgsign=false commit -m "base"

    # Stage a misformatted ts change that Prettier will fix
    printf 'export function f(){return 2}' > file.ts
    blob=$(printf 'export function f(){return 2}' | git hash-object -w --stdin)
    git update-index --cacheinfo 100644 "$blob" file.ts

    # Make an unrelated UNSTAGED change to lockfile
    printf 'name: test # foo\n' > pnpm-lock.yaml

    # Sanity: staged has only file.ts, unstaged has pnpm-lock.yaml
    run bash -lc "git diff --staged --name-only"
    assert_line 'file.ts'
    run bash -lc "git diff --name-only"
    assert_line 'pnpm-lock.yaml'
}

@test "pre-commit should not include unstaged lockfile edits in commit" {
    create_precommit_with_prettier
    prepare_repo_with_staged_ts_and_unstaged_lockfile

    # Run a real commit to invoke hk pre-commit
    run git -c commit.gpgsign=false commit -m "test"
    assert_success

    # HEAD must contain only the ts change (after Prettier), not the lockfile edit
    run bash -lc "git show --name-only --pretty=format: HEAD"
    refute_line 'pnpm-lock.yaml'
    assert_line 'file.ts'

    # The unstaged lockfile change should remain in the worktree
    run bash -lc "git diff --name-only"
    assert_line 'pnpm-lock.yaml'

    # Ensure no stash remains after pre-commit completes
    run bash -lc 'git stash list'
    assert_success
    assert_output ""
}

