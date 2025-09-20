#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Minimal pre-commit that does not modify files but triggers stash/unstash flow
create_minimal_precommit() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps = new Mapping<String, Step> {
      ["noop"] {
        glob = "*.txt"
        stage = "*.txt"
        fix = "bash -lc 'true'"
      }
    }
  }
}
PKL
    git add hk.pkl
    git -c commit.gpgsign=false commit -m "init hk"
    hk install
}

prepare_staged_with_unstaged_newline_only_change() {
    # Base commit
    printf 'base\n' > t.txt
    git add t.txt
    git -c commit.gpgsign=false commit -m "base"

    # Stage new content WITHOUT trailing newline in the index
    printf '%s' "staged-content" > t.txt   # no trailing newline
    git add t.txt

    # Introduce an UNSTAGED change that is only a trailing newline at EOF
    # (worktree differs from index by a single '\n')
    printf '\n' >> t.txt  # now worktree has a trailing newline

    # Sanity: diff should report the missing newline marker
    run bash -lc "git diff -- t.txt"
    assert_success
    assert_output --partial "No newline at end of file"
}

@test "pre-commit preserves exact EOF newline state; does not add/remove trailing newline" {
    create_minimal_precommit
    prepare_staged_with_unstaged_newline_only_change

    # Run hook explicitly
    run bash -lc 'HK_LOG=debug HK_SUMMARY_TEXT=1 hk run pre-commit || true'
    echo "$output"

    # After stash/unstash, hk should NOT modify EOF newline state of the worktree.
    # The worktree should still differ from index only by the trailing newline.
    run bash -lc "git diff -- t.txt"
    assert_success
    assert_output --partial "No newline at end of file"
}
