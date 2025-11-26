#!/usr/bin/env mise run test:bats

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
    git commit -m "init hk"
    hk install
}

# Prepare file with a staged, misformatted change that prettier will fix, and an unstaged line
prepare_staged_misformatted_with_unstaged_tail() {
    # Base commit
    cat <<'TS' > file.ts
export function f() { return 0; }
TS
    git add file.ts
    git commit -m "base"

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
    run bash -lc "git diff --staged -- file.ts"
    assert_line --partial 'export function f(){return 2}'
    run bash -lc "git diff -- file.ts"
    assert_line --partial '// unstaged'
}

@test "pre-commit (stash=git) commits prettier-fixed staged change (not misformatted) and preserves unstaged" {
    create_precommit_prettier_with_stash git
    prepare_staged_misformatted_with_unstaged_tail

    # Run real commit to trigger hk pre-commit and fixers
    run git commit -m "test"
    assert_success

    # HEAD must contain the PRETTIER-FIXED variant, not the misformatted one
    run bash -lc "git show HEAD:file.ts"
    assert_line --partial 'export function f() {'
    assert_line --partial 'return 2;'
    refute_line --partial 'export function f(){return 2}'
    refute_line --partial '// unstaged'

    # Worktree should still contain the unstaged marker
    run bash -lc "cat file.ts"
    assert_line --partial '// unstaged'

    # Index should be clean after commit
    run bash -lc "git diff --staged --name-only"
    assert_output ""
}

@test "pre-commit (stash=patch-file) commits prettier-fixed staged change (not misformatted) and preserves unstaged" {
    create_precommit_prettier_with_stash patch-file
    prepare_staged_misformatted_with_unstaged_tail

    # Run real commit to trigger hk pre-commit and fixers
    run git commit -m "test"
    assert_success

    # HEAD must contain the PRETTIER-FIXED variant, not the misformatted one
    run bash -lc "git show HEAD:file.ts"
    assert_line --partial 'export function f() {'
    assert_line --partial 'return 2;'
    refute_line --partial 'export function f(){return 2}'
    refute_line --partial '// unstaged'

    # Worktree should still contain the unstaged marker
    run bash -lc "cat file.ts"
    assert_line --partial '// unstaged'

    # Index should be clean after commit
    run bash -lc "git diff --staged --name-only"
    assert_output ""
}

# Scenario mirroring real-world bug: staged top-of-file insertion, Prettier adds a semicolon,
# and an unrelated unstaged tail. The fixer change (semicolon) SHOULD be committed.
prepare_top_insertion_and_unstaged_tail_with_prettier_fix() {
    # Base commit with properly formatted import (with semicolon)
    cat <<'TS' > file.ts
import x from 'a';
TS
    git add file.ts
    git -c commit.gpgsign=false commit -m "base"

    # Create worktree content missing the semicolon so Prettier will fix it,
    # plus an unstaged tail line
    cat <<'TS' > file.ts
// staged
import x from 'a'
// unstaged
TS

    # Stage ONLY the top comment + import line (without the trailing unstaged line)
    cat <<'TS' > .staged
// staged
import x from 'a'
TS
    blob=$(git hash-object -w --stdin < .staged)
    rm .staged
    git update-index --cacheinfo 100644 "$blob" file.ts

    # Sanity: staged has no semicolon; worktree has the tail line
    run bash -lc "git show :file.ts"
    refute_line --partial ';'
    run bash -lc "git diff -- file.ts"
    assert_line --partial '// unstaged'
}

@test "pre-commit (stash=git) should commit Prettier semicolon even with top insertion + unstaged tail" {
    create_precommit_prettier_with_stash git
    prepare_top_insertion_and_unstaged_tail_with_prettier_fix

    run git -c commit.gpgsign=false commit -m "test"
    assert_success

    # EXPECTATION: HEAD contains the semicolon added by Prettier
    run bash -lc "git show HEAD:file.ts"
    assert_line "import x from \"a\";"

    # And HEAD should not include the unstaged tail
    refute_line --partial '// unstaged'
}

@test "pre-commit (stash=patch-file) should commit Prettier semicolon even with top insertion + unstaged tail" {
    create_precommit_prettier_with_stash patch-file
    prepare_top_insertion_and_unstaged_tail_with_prettier_fix

    run git -c commit.gpgsign=false commit -m "test"
    assert_success

    run bash -lc "git show HEAD:file.ts"
    assert_line "import x from \"a\";"
    refute_line --partial '// unstaged'
}
