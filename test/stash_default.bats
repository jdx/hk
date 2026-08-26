#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "stash default for pre-commit" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        steps {
            ["trailing-whitespace"] = Builtins.trailing_whitespace
        }
    }
}
PKL
    git add hk.pkl
    git commit -m "Required initial commit"
    echo "content  " > file.txt
    git add file.txt
    echo "changed content  " > file.txt

    run hk run pre-commit
    assert_success
    refute_output --partial "Stashed unstaged changes"
    run cat -e file.txt
    assert_success
    assert_output "changed content$"
}

@test "stash default for fix" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["trailing-whitespace"] = Builtins.trailing_whitespace
        }
    }
}
PKL
    git add hk.pkl
    git commit -m "Required initial commit"
    echo "content  " > file.txt
    git add file.txt
    echo "changed content  " > file.txt

    run hk run fix
    refute_output --partial "Stashed unstaged changes"
    run cat -e file.txt
    assert_success
    assert_output "changed content$"
}
