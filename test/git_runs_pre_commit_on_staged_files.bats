#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "git runs pre-commit on staged files" {
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
    git add hk.pkl
    git commit -m "init"
    cat <<EOF > test.js
console.log("test")
EOF
    run git add test.js
    hk install
    run cat test.js
    assert_output 'console.log("test")'
    git commit -m "test"
    run cat test.js
    assert_output 'console.log("test");'
}

