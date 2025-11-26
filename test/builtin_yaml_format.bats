#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtin: yaml format" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "patch-file"
        steps {
            ["yq"] = Builtins.yq
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"
    cat <<EOF > test.yaml
    test: 123
EOF
    git add test.yaml
    cat test.yaml
    hk run pre-commit
    assert_file_contains test.yaml 'test: 123'
}
