#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Minimal hk pre-commit that runs prettier with fix enabled
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

# Prepare file mirroring the observed scenario: a staged change that Prettier will fix
# plus an unrelated unstaged tail. After commit, the fixer change should be present
# in HEAD and also not be reverted in the worktree.
prepare_top_insertion_and_unstaged_tail_with_prettier_fix() {
    # Base commit with properly formatted import (with semicolon)
    cat <<'TS' > file.ts
import x from 'a';
TS
    git add file.ts
    git -c commit.gpgsign=false commit -m "base"

    # Worktree: top comment + import missing semicolon so Prettier will fix it, plus an unstaged tail
    cat <<'TS' > file.ts
// staged
import x from 'a'
// unstaged
TS

    # Stage ONLY the top portion (without the unstaged tail)
    cat <<'TS' > .staged
// staged
import x from 'a'
TS
    blob=$(git hash-object -w --stdin < .staged)
    rm .staged
    git update-index --cacheinfo 100644 "$blob" file.ts

    # Sanity: staged lacks semicolon; worktree has the tail line
    run bash -lc "git show :file.ts"
    refute_line --partial ';'
    run bash -lc "git diff -- file.ts"
    assert_line --partial '// unstaged'
}

@test "pre-commit restores worktree content that reverts Prettier fix (regression)" {
    # Use git-based stashing to reflect default observed behavior
    create_precommit_prettier_with_stash git
    prepare_top_insertion_and_unstaged_tail_with_prettier_fix

    # Perform a real commit which triggers hk pre-commit and Prettier
    run git -c commit.gpgsign=false commit -m "test"
    assert_success

    # HEAD should contain the PRETTIER-FIXED import (with semicolon)
    run bash -lc "git show HEAD:file.ts"
    assert_line "import x from \"a\";"
    refute_line --partial '// unstaged'

    # EXPECTATION for this test (intentionally failing given current behavior):
    # The worktree should NOT revert the Prettier fix. Verify that worktree also has the semicolon.
    run bash -lc "cat file.ts"
    assert_line "import x from \"a\";"
}

