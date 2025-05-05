setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "patch-file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        stash = "git"
        steps {
            ["append"] {
                glob = "*"
                stage = "*"
                check = """
for file in {{files}}; do
    echo "patch" > \$file
done
"""
            }
        }
    }
}
EOF
    touch a b c
    git add hk.pkl a b c
    git commit -m "initial commit"
    echo "a" > a
    echo "b" > b
    echo "c" > c
    git add a b
    echo "b" >> b
    hk run pre-commit
    run cat a
    assert_output "patch"
    run cat b
    assert_output "b
b"
    run cat c
    assert_output "c"
}
