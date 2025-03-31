setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "prepare-commit-msg hook" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks = new {
    ["prepare-commit-msg"] {
        steps {
            ["render-commit-msg"] {
                run = "echo default_commit_msg > {{commit_msg_file}}"
            }
        }
    }
}
EOF
    hk install
    echo "test" > test.txt
    git add test.txt
    run git commit --no-edit
    assert_output --partial "default_commit_msg"
}
