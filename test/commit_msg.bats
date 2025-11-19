setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "commit-msg hook" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks = new {
    ["commit-msg"] {
        steps {
            ["validate-commit-msg"] {
                check = "grep -q '^feat: ' {{commit_msg_file}} || (echo 'Commit message must start with feat:' >&2 && exit 1)"
            }
        }
    }
}
EOF
    hk install
    echo "test" > test.txt
    git add test.txt
    run git commit -m "test"
    assert_failure
    assert_output --partial "Commit message must start with feat:"

    run git commit -m "feat: add test file"
    assert_success
}
