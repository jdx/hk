#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

create_config_with_stash() {
    local stash_method="$1"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    stash = "$stash_method"
    steps {
      ["format"] {
        glob = "conflict.txt"
        check = "grep -q 'formatted' conflict.txt || exit 1"
        fix = "printf 'formatted\\n' > conflict.txt"
        stage = "conflict.txt"
      }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init config"
}

@test "stash=git: handles apply conflicts robustly" {
    create_config_with_stash git

    # Set up a scenario that could cause stash apply issues
    printf 'original\n' > conflict.txt
    git add conflict.txt
    git commit -m "base"

    # Stage changes
    printf 'staged\n' > conflict.txt
    git add conflict.txt

    # Make conflicting unstaged changes
    printf 'unstaged\n' > conflict.txt

    # Add untracked files to test preservation
    printf 'untracked\n' > untracked.txt

    # This should succeed without errors
    run hk fix
    assert_success

    # Verify repository is in a consistent state
    run git status
    assert_success

    # Verify unstaged content is preserved (core fix behavior)
    run grep -q 'unstaged' conflict.txt
    assert_success

    # Verify untracked files are preserved
    test -f untracked.txt

    # Verify no conflict markers left
    run grep -E '^(<<<<<<<|=======|>>>>>>>)' conflict.txt
    assert_failure
}

@test "stash=patch-file: handles apply conflicts robustly" {
    create_config_with_stash patch-file

    # Set up same scenario for patch-file mode
    printf 'original\n' > conflict.txt
    git add conflict.txt
    git commit -m "base"

    # Stage changes
    printf 'staged\n' > conflict.txt
    git add conflict.txt

    # Make conflicting unstaged changes
    printf 'unstaged\n' > conflict.txt

    # Add untracked files to test preservation
    printf 'untracked\n' > untracked.txt

    # This should succeed without errors
    run hk fix
    assert_success

    # Verify repository is in a consistent state
    run git status
    assert_success

    # Verify unstaged content is preserved
    run grep -q 'unstaged' conflict.txt
    assert_success

    # Verify untracked files are preserved
    test -f untracked.txt

    # Verify no conflict markers left
    run grep -E '^(<<<<<<<|=======|>>>>>>>)' conflict.txt
    assert_failure
}

@test "stash=git: multiple operations work correctly" {
    create_config_with_stash git

    printf 'original\n' > conflict.txt
    git add conflict.txt
    git commit -m "base"

    # First operation
    printf 'staged1\n' > conflict.txt
    git add conflict.txt
    printf 'unstaged1\n' > conflict.txt

    run hk fix
    assert_success

    # Second operation
    printf 'staged2\n' > conflict.txt
    git add conflict.txt
    printf 'unstaged2\n' > conflict.txt

    run hk fix
    assert_success

    # Final check
    run grep -q 'unstaged2' conflict.txt
    assert_success
}

@test "stash error handling: no crashes on edge cases" {
    create_config_with_stash git

    # Create edge case: empty repository initially
    printf 'new file\n' > new.txt

    # Try to run hk fix on untracked file
    run hk fix
    # Should not crash, may succeed or fail gracefully
    [[ $status -eq 0 || $status -eq 1 ]]

    # Repository should still be usable
    run git status
    assert_success
}
