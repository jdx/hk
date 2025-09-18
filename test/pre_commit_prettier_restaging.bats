#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Create a minimal hk pre-commit that runs prettier with fix enabled
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
    git -c commit.gpgsign=false commit -m "init hk"
    hk install
}

# Prepare file with a staged, misformatted change that prettier will fix, and an unstaged line
prepare_staged_misformatted_with_unstaged_tail() {
    # Base commit
    cat <<'TS' > file.ts
export function f() { return 0; }
TS
    git add file.ts
  env HK=0 git -c commit.gpgsign=false commit -m "base"

  # Diagnostics: show repo state after base commit
  run bash -lc "git log --oneline -n 5"
  echo "$output"
  run bash -lc "git status --porcelain -z | tr '\0' '\n'"
  echo "$output"

    # Working tree has misformatted change PLUS an extra unstaged line
    cat <<'TS' > file.ts
export function f(){return 2}
// unstaged
TS

    # Stage ONLY the misformatted change by writing a blob directly to the index
    printf '%s\n' "export function f(){return 2}" | git hash-object -w --stdin >.blob
    blob=$(cat .blob)
    rm .blob
    git update-index --cacheinfo 100644 "$blob" file.ts

    # Sanity: staged shows misformatted variant, unstaged shows the trailing comment
    run bash -lc "git diff --staged -- file.ts | grep -F 'export function f(){return 2}'"
    assert_success
    run bash -lc "git diff -- file.ts | grep -F '// unstaged'"
    assert_success
}

@test "pre-commit (stash=git) commits prettier-fixed staged change and preserves unstaged" {
    create_precommit_prettier_with_stash git
    prepare_staged_misformatted_with_unstaged_tail

    # Run hook explicitly to allow us to inspect and avoid reentrancy issues
    run bash -lc 'set -x; HK_LOG=debug HK_SUMMARY_TEXT=1 hk run pre-commit || true'
    echo "$output"
    run bash -lc '[ -f "$HK_STATE_DIR/output.log" ] && { echo "==== HK output.log ===="; cat "$HK_STATE_DIR/output.log"; } || true'
    # Verify INDEX has PRETTIER-FIXED variant (spaces and semicolon), not the misformatted one
    run bash -lc "git show :file.ts | grep -F 'export function f() { return 2; }'"
    assert_success
    run bash -lc "git show :file.ts | grep -F 'export function f(){return 2}'"
    assert_failure

    # Index must not contain the unstaged marker
    run bash -lc "git show :file.ts | grep -F '// unstaged'"
    assert_failure

    # Worktree should still contain the unstaged marker
    run grep -Fq "// unstaged" file.ts
    assert_success

    # The file should be staged for commit
    run bash -lc "git diff --staged --name-only"
    assert_output "file.ts"
}

@test "pre-commit (stash=patch-file) commits prettier-fixed staged change and preserves unstaged" {
    create_precommit_prettier_with_stash patch-file
    prepare_staged_misformatted_with_unstaged_tail

    run bash -lc 'set -x; HK_LOG=debug HK_SUMMARY_TEXT=1 hk run pre-commit || true'
    echo "$output"
    run bash -lc '[ -f "$HK_STATE_DIR/output.log" ] && { echo "==== HK output.log ===="; cat "$HK_STATE_DIR/output.log"; } || true'

    run bash -lc "git show :file.ts | grep -F 'export function f() { return 2; }'"
    assert_success
    run bash -lc "git show :file.ts | grep -F 'export function f(){return 2}'"
    assert_failure
    run bash -lc "git show :file.ts | grep -F '// unstaged'"
    assert_failure
    run grep -Fq "// unstaged" file.ts
    assert_success
    run bash -lc "git diff --staged --name-only"
    assert_output "file.ts"
}
