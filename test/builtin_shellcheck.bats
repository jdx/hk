#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtin: shellcheck" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["shellcheck"] = Builtins.shellcheck
        }
    }
}
EOF
    cat <<EOF > test.sh
#!/bin/bash
cat \$1
EOF
    git add hk.pkl
    git commit -m "init"
    git add test.sh
    run hk run pre-commit
    assert_failure
    assert_output --partial "SC2086"
}
