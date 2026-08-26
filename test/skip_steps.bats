#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "HK_SKIP_STEPS skips specified steps" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
display_skip_reasons = List("disabled-by-env")
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["prettier"] = Builtins.prettier
            ["newlines"] = Builtins.newlines
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"
    touch test.sh
    touch test.js
    git add test.sh test.js
    export HK_SKIP_STEPS="newlines"
    run hk run pre-commit -v
    assert_success
    assert_output --partial "prettier"
    assert_output --partial "newlines – skipped: disabled via HK_SKIP_STEPS"
}
