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

@test "unstaged changes get restored" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["succeed"] { check = "exit 0" }
            ["fail"] { check = "exit 1" }
        }
    }
}
EOF
    mkdir -p src
    touch src/foo.rs
    git add hk.pkl src/foo.rs
    git commit -m "initial commit"
    hk install

    echo "staged" >> src/foo.rs
    git add src/foo.rs
    echo "unstaged" >> src/foo.rs
    run git commit -m "staged changes"
    assert_failure
    run cat src/foo.rs
    assert_output "staged
unstaged"
    run git diff
    assert_output --partial "staged
+unstaged"
}
