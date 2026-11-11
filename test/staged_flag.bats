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

    run hk_stash_list
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

@test "hk run pre-commit --from-hook --staged honors the hook's stash setting" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
steps {
    ["upper"] {
        glob = "*.txt"
        fix = "perl -pi -e 's/foo/FOO/g' {{files}}"
    }
}
EOF
    git add hk.pkl
    git commit -m "init"

    printf '1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n' > a.txt
    git add a.txt
    git commit -m "add a"

    printf '1 foo\n2\n3\n4\n5\n6\n7\n8\n9\n10 unstaged\n' > a.txt
    printf 'y\nn\n' | git add -p a.txt > /dev/null

    HK_STASH=none run hk run pre-commit --from-hook --staged
    assert_success

    run git diff --cached
    assert_output --partial "+1 FOO"
    refute_output --partial "10 unstaged"

    run git diff
    assert_output --partial "+10 unstaged"
}

@test "hk run pre-commit --from-hook --staged lets a hook opt out of stashing despite HK_STASH=git" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "none"
        steps {
            ["upper"] {
                glob = "*.txt"
                fix = "perl -pi -e 's/foo/FOO/g' {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"

    printf '1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n' > a.txt
    git add a.txt
    git commit -m "add a"

    printf '1 foo\n2\n3\n4\n5\n6\n7\n8\n9\n10 unstaged\n' > a.txt
    printf 'y\nn\n' | git add -p a.txt > /dev/null

    HK_STASH=git run hk run pre-commit --from-hook --staged
    assert_success

    # No stash happened, so the fixer saw the whole file and restaged it.
    run git diff --cached
    assert_output --partial "+10 unstaged"
    run hk_stash_list
    assert_output ""
}
