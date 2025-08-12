#!/usr/bin/env bats

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
  ["fix"] {
    fix = true
    stash = "$method"
    steps {
      ["overwrite"] {
        glob = "conflict.txt"
        fix = "printf 'line1\\nfixed\\nline3\\n' > conflict.txt"
        stage = "conflict.txt"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
}

prepare_conflict_state() {
    # Base file and commit
    printf 'line1\nline2\nline3\n' > conflict.txt
    git add conflict.txt
    git commit -m "base"

    # Stage a change to the same line
    printf 'line1\nstaged-change\nline3\n' > conflict.txt
    git add conflict.txt

    # Make an unstaged change to the same line
    printf 'line1\nunstaged-change\nline3\n' > conflict.txt
}

assert_unstaged_preferred() {
    # After hk fix, we should prefer original unstaged contents
    assert_file_contains conflict.txt 'unstaged-change'
    run grep -q 'fixed' conflict.txt
    assert_failure
    run grep -q '<<<<<<<' conflict.txt
    assert_failure
    run grep -q '>>>>>>>' conflict.txt
    assert_failure
}

@test "stash=git: prefer unstaged over fixer changes on same lines" {
    create_config_with_stash_method git
    prepare_conflict_state
    hk fix -v
    assert_unstaged_preferred
}

@test "stash=patch-file: prefer unstaged over fixer changes on same lines" {
    create_config_with_stash_method patch-file
    prepare_conflict_state
    hk fix -v
    assert_unstaged_preferred
}
