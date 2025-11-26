#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk run pre-commit --all runs on all files" {
    cat <<EOF > test.js
console.log("test")
EOF
    git add test.js
    git commit -m init
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["prettier"] = Builtins.prettier
        }
    }
}
EOF
    hk run pre-commit --all
    assert_file_exists hk.pkl
    run cat test.js
    assert_output 'console.log("test");'
}

