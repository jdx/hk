#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtin: json format" {
    setup_with_builtin jq "
hooks {
    [\"pre-commit\"] {
        fix = true
        stash = \"git\"
        steps {
            [\"jq\"] = Builtins.jq
        }
    }
}"

    git add hk.pkl
    git commit -m "init"

    cat <<EOF > test.json
{"test": 123}
EOF
    git add test.json

    assert_hk_success run pre-commit
    assert_file_contains test.json '{
  "test": 123
}'
} 
