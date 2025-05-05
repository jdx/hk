setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "git" {
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
    run hk run pre-commit
    run cat a
    assert_output "<<<<<<< Updated upstream
a
=======
patch
>>>>>>> Stashed changes"
    run cat b
    assert_output "<<<<<<< Updated upstream
b
b
=======
patch
>>>>>>> Stashed changes"
    run cat c
    assert_output "c"
}

@test "patch-file" {
    skip "patch-file needs to support merge conflicts"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        stash = "patch-file"
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
    assert_output "<<<<<<< Updated upstream
a
=======
patch
>>>>>>> Stashed changes"
    run cat b
    assert_output "<<<<<<< Updated upstream
b
b
=======
patch
>>>>>>> Stashed changes"
    run cat c
    assert_output "c"
}
