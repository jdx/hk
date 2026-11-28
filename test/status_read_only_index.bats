#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# A run that neither stashes nor stages reads git status without writing the
# refreshed index back, so it never takes `.git/index.lock` from git commands
# other tools run in the same worktree. git replaces the index by renaming
# `index.lock` over it, which gives it a new inode.
setup_repo() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        fix = false
        stage = false
        stash = "none"
        steps { ["noop"] { check = "true" } }
    }
    ["pre-commit"] {
        fix = false
        stage = false
        stash = "none"
        steps { ["noop"] { check = "true" } }
    }
}
EOF
    echo hello > a.txt
    git add a.txt hk.pkl
    git commit -q -m init
    # Make a.txt's stat information stale without changing its contents.
    # `touch -t` takes the same format with GNU and BSD touch.
    touch -t 203001010000 a.txt
}

index_inode() {
    ls -i .git/index | awk '{print $1}'
}

@test "read-only hk check does not rewrite the index" {
    setup_repo
    unset GIT_OPTIONAL_LOCKS
    before=$(index_inode)
    run hk check
    assert_success
    assert_equal "$(index_inode)" "$before"
}

@test "read-only hk run pre-commit does not rewrite the index" {
    setup_repo
    unset GIT_OPTIONAL_LOCKS
    before=$(index_inode)
    run hk run pre-commit
    assert_success
    assert_equal "$(index_inode)" "$before"
}
