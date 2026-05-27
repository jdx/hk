#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk run pre-commit --staged only passes staged files and does not stash" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        stash = true
        steps {
            ["capture"] {
                check = "printf '%s\n' {{files}} > seen.txt"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"

    echo "tracked" > tracked.txt
    git add tracked.txt
    git commit -m "add tracked"

    echo "unstaged" >> tracked.txt
    echo "staged" > staged.txt
    git add staged.txt
    echo "untracked" > untracked.txt

    run hk run pre-commit --staged
    assert_success

    run cat seen.txt
    assert_output "staged.txt"

    run cat tracked.txt
    assert_output $'tracked\nunstaged'

    assert_file_exists untracked.txt

    run git stash list
    assert_output ""
}

@test "hk run pre-commit --staged conflicts with explicit stash option" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["check"] {
                check = "true"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"

    run hk run pre-commit --staged --stash git
    assert_failure
    assert_output --partial "cannot be used with"
}
