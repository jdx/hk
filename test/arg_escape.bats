#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "arg escape" {
    export NO_COLOR=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks { ["pre-commit"] { steps { ["prettier"] = Builtins.prettier } } }
EOF
    git add hk.pkl
    git commit -m "install hk"
    hk install
    echo 'console.log("test")' > '$test.js'
    git add '$test.js'
    run git commit -m "test"
    assert_failure
    assert_output --partial '[warn] $test.js'
}
