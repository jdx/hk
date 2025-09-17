#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Reproduces: with partially staged changes in a single file, running
# `git commit` (which triggers hk pre-commit with stashing + prettier)
# should commit only the staged hunk and leave the unstaged hunk in the
# working tree. This currently regresses (staged hunk not committed).

create_precommit_prettier_with_stash() {
    local method="$1"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "$method"
    steps {
      ["prettier"] = Builtins.prettier
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init hk"
    hk install
}

prepare_partially_staged_ts_file() {
    # Base commit
    cat <<'TS' > file.ts
export function f(){return 1}
TS
    git add file.ts
    git commit -m "base"

    # Working tree should have BOTH the staged change (return 2) and an
    # additional unstaged line added. We'll put both in the worktree first.
    cat <<'TS' > file.ts
export function f(){return 2}
// foo
TS

    # Now stage ONLY the "return 2" change by writing a blob directly to index
    # that does NOT include the trailing comment line.
    printf '%s\n' "export function f(){return 2}" | git hash-object -w --stdin >.blob
    blob=$(cat .blob)
    rm .blob
    git update-index --cacheinfo 100644 "$blob" file.ts

    # Sanity checks: staged diff has return 2, unstaged diff has // foo
    run bash -lc "git diff --staged -- file.ts | grep -F 'return 2'"
    assert_success
    run bash -lc "git diff -- file.ts | grep -F '// foo'"
    assert_success
}

@test "pre-commit with stash=git commits staged hunk and preserves unstaged hunk" {
    create_precommit_prettier_with_stash git
    prepare_partially_staged_ts_file

    # Run commit to trigger hk pre-commit
    run git commit -m "test"
    assert_success

    # HEAD should include the staged change (return 2)
    run bash -lc "git show HEAD:file.ts | grep -F 'return 2'"
    assert_success

    # HEAD should NOT include the unstaged line
    run bash -lc "git show HEAD:file.ts | grep -F '// foo'"
    assert_failure

    # Worktree should still contain the unstaged line
    run grep -Fq "// foo" file.ts
    assert_success

    # Index should be clean for this path post-commit
    run bash -lc "git diff --staged --name-only"
    assert_output ""
}

@test "pre-commit with stash=patch-file commits staged hunk and preserves unstaged hunk" {
    create_precommit_prettier_with_stash patch-file
    prepare_partially_staged_ts_file

    # Run commit to trigger hk pre-commit
    run git commit -m "test"
    assert_success

    # HEAD should include the staged change (return 2)
    run bash -lc "git show HEAD:file.ts | grep -F 'return 2'"
    assert_success

    # HEAD should NOT include the unstaged line
    run bash -lc "git show HEAD:file.ts | grep -F '// foo'"
    assert_failure

    # Worktree should still contain the unstaged line
    run grep -Fq "// foo" file.ts
    assert_success

    # Index should be clean for this path post-commit
    run bash -lc "git diff --staged --name-only"
    assert_output ""
}
