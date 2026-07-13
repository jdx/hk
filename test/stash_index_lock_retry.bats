#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

assert_stash_waits_for_transient_index_lock() {
    local use_libgit2="$1"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["check"] {
        glob = "*.txt"
        check = "true"
      }
    }
  }
}
EOF

    printf 'staged v1\n' > staged.txt
    printf 'unstaged v1\n' > unstaged.txt
    git add hk.pkl staged.txt unstaged.txt
    git commit -m "init"

    printf 'staged v2\n' > staged.txt
    printf 'unstaged v2\n' > unstaged.txt
    git add staged.txt

    : > .git/index.lock
    (sleep 0.2; rm -f .git/index.lock) &

    run env HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_success

    run git diff -- unstaged.txt
    assert_success
    assert_output --partial "+unstaged v2"

    run git diff --cached -- staged.txt
    assert_success
    assert_output --partial "+staged v2"

    run git stash list
    assert_success
    assert_output ""
}

@test "partial-path git stash waits for a transient index lock with libgit2 enabled" {
    assert_stash_waits_for_transient_index_lock 1
}

@test "git stash waits for a transient index lock with libgit2 disabled" {
    assert_stash_waits_for_transient_index_lock 0
}
