#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "pre-commit defaults to check" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["trailing-whitespace"] = Builtins.trailing_whitespace
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "content  " > file.txt
    git add file.txt

    run hk run pre-commit
    assert_failure

    run cat -e file.txt
    assert_success
    assert_output "content  $"
}

@test "fix defaults to fix" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["fix"] {
        steps {
            ["trailing-whitespace"] = Builtins.trailing_whitespace
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "content  " > file.txt
    git add file.txt

    run hk run fix
    assert_success

    run cat -e file.txt
    assert_success
    assert_output "content$"
}

