#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtin: shellcheck" {
    setup_with_config 'amends "'"$PKL_PATH"'/Config.pkl"
import "'"$PKL_PATH"'/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["shellcheck"] = Builtins.shellcheck
        }
    }
}'
    cat <<EOF > test.sh
#!/bin/bash
cat \$1
EOF
    git add hk.pkl
    git commit -m "init"
    git add test.sh

    assert_hk_failure run pre-commit
    assert_output --partial "SC2086"
} 
