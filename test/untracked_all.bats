#!/usr/bin/env mise run test:bats
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "untracked changes are tested without --all" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { check = "echo files: {{files}}" }
        }
    }
}
EOF
    git init
    git add hk.pkl
    git commit -m "initial commit"
    mkdir -p src
    touch src/foo.rs
    touch src/bar.rs
    touch root.rs
    run hk check
    assert_success
    assert_output --partial "files: root.rs src/bar.rs src/foo.rs"
}

@test "untracked changes are tested with --all" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { check = "echo files: {{files}}" }
        }
    }
}
EOF
    git init
    git add hk.pkl
    git commit -m "initial commit"
    mkdir -p src
    touch src/foo.rs
    touch src/bar.rs
    touch root.rs
    run hk check --all
    assert_success
    assert_output --partial "files: hk.pkl root.rs src/bar.rs src/foo.rs"
}

