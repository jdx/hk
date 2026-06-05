#!/usr/bin/env bats

# Regression test for https://github.com/jdx/hk/discussions/965
# A file with both staged and unstaged hunks where the unstaged edit touches the
# LAST line must be restored intact after pre-commit. Previously the stash
# restore misclassified the last-line edit as a "pure tail insertion" and split
# it onto a new line with a leading space.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

create_config_with_stash_method() {
    local method="$1"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "$method"
    steps {
      ["noop"] {
        glob = "*.txt"
        check_first = false
        fix = "true"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
}

prepare_partially_staged_last_line_edit() {
    # Base commit
    printf 'l1\nl2\nl3: tail\n' > f.txt
    git add f.txt
    git commit -m "base"

    # Stage an edit to the first line
    printf 'l1 STAGED\nl2\nl3: tail\n' > f.txt
    git add f.txt

    # Unstaged edit to the LAST line
    printf 'l1 STAGED\nl2\nl3: tail UNSTAGED\n' > f.txt
}

assert_last_line_restored_intact() {
    run cat f.txt
    assert_output "l1 STAGED
l2
l3: tail UNSTAGED"
}

@test "stash=git: unstaged edit to last line of partially-staged file survives pre-commit" {
    create_config_with_stash_method git
    prepare_partially_staged_last_line_edit
    hk run pre-commit
    assert_last_line_restored_intact
}

@test "stash=patch-file: unstaged edit to last line of partially-staged file survives pre-commit" {
    create_config_with_stash_method patch-file
    prepare_partially_staged_last_line_edit
    hk run pre-commit
    assert_last_line_restored_intact
}

@test "stash backup patch is valid for git apply" {
    export HK_STATE_DIR="$TEST_TEMP_DIR/hk-state"
    create_config_with_stash_method git
    prepare_partially_staged_last_line_edit
    hk run pre-commit

    patch=$(ls "$HK_STATE_DIR"/patches/*.patch | head -1)
    assert [ -n "$patch" ]
    # The backup patch must parse (it previously lacked a trailing newline,
    # which git apply rejects as a corrupt patch)
    run git apply --numstat "$patch"
    assert_success
}
