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

@test "binary files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["binary"] { check = "echo 'binary: {{files}}'"; attributes = List("binary") }
            ["text"] { check = "echo 'text: {{files}}'"; attributes = List("text") }
            ["all"] { check = "echo 'all: {{files}}'" }
        }
    }
}
EOF
    cat <<EOF >.gitattributes
binary.txt binary
text.txt text
EOF
    echo "binary" > binary.txt
    echo "text" > text.txt
    git add hk.pkl .gitattributes
    git commit -m "initial commit"
    git add binary.txt text.txt
    git commit -m "add binary and text"
    run hk check --from-ref HEAD^
    assert_success
    assert_output --partial "binary: binary.txt"
    assert_output --partial "text: text.txt"
    assert_output --partial "all: binary.txt text.txt"
}
