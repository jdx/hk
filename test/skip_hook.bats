#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "HK_SKIP_HOOK skips entire hooks" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "patch-file"
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
    export HK_SKIP_HOOK="pre-commit"
    run hk run pre-commit -v
    assert_success
    assert_output --partial "pre-commit: skipping hook due to HK_SKIP_HOOK"
}
