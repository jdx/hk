#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtin: json format" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["jq"] = Builtins.jq
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"
    cat <<EOF > test.json
{"test": 123}
EOF
    git add test.json
    hk run pre-commit
    assert_file_contains test.json '{
  "test": 123
}'
}
