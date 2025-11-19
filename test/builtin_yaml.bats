#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtin: yaml" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["yq"] = Builtins.yq
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"
    cat <<EOF > test.yaml
test: :
EOF
    git add test.yaml
    run hk run pre-commit
    assert_failure
    assert_output --partial "yaml: mapping values are not allowed"
}
