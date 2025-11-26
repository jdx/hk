#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtin: json" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "patch-file"
        steps {
            ["jq"] = Builtins.jq
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"
    cat <<EOF > test.json
{ "invalid":
EOF
    git add test.json
    run hk run pre-commit
    assert_failure
    assert_output --partial "parse error"
}

