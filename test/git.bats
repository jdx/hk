setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "commit-a" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["foo"] {
                check = "echo 'foo: {{files}}'"
            }
        }
    }
}
EOF
    mkdir -p src
    touch src/foo.rs
    git add hk.pkl src/foo.rs
    git commit -m "initial commit"
    hk install

    echo "text" > src/foo.rs
    run git commit -am "add text"
    assert_success
    assert_output --partial "foo: src/foo.rs"
}
