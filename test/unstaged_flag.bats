#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk run check --unstaged passes only unstaged and untracked files and does not stash" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["capture"] {
                check = "printf '%s\n' {{files}} | sort > seen.txt"
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

    # tracked.txt: modified in working tree (unstaged)
    echo "unstaged" >> tracked.txt
    # staged.txt: staged only -> should be excluded
    echo "staged" > staged.txt
    git add staged.txt
    # untracked.txt: untracked -> should be included
    echo "untracked" > untracked.txt

    run hk check --unstaged
    assert_success

    run cat seen.txt
    assert_output $'tracked.txt\nuntracked.txt'

    run git stash list
    assert_output ""
}

@test "hk run check --unstaged conflicts with --staged" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
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

    run hk check --unstaged --staged
    assert_failure
}

@test "hk run check --unstaged conflicts with explicit stash option" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
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

    run hk check --unstaged --stash git
    assert_failure
    assert_output --partial "cannot be used with"
}
