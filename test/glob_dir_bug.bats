#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "glob with dir maintains correct path semantics" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test-step"] {
                dir = "src"
                glob = List("**/*.rs")
                check = "echo files {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src/subdir
    echo "fn main() {}" > src/main.rs
    echo "fn test() {}" > src/subdir/test.rs

    git add src/main.rs src/subdir/test.rs

    run hk check
    assert_success

    # Files should be shown relative to dir (no src/ prefix)
    assert_output --partial "files main.rs subdir/test.rs"
}

@test "glob pattern with dir matches correctly" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["test-step"] {
                dir = "project"
                glob = List("src/**/*.js")
                check = "echo files {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p project/src/components
    mkdir -p project/tests
    echo "export const x = 1;" > project/src/components/Button.js
    echo "export const y = 2;" > project/src/index.js
    echo "test();" > project/tests/test.js

    git add project/src/components/Button.js project/src/index.js project/tests/test.js

    run hk check
    assert_success

    # Should match files in project/src/**/*.js and display them relative to project/
    assert_output --partial "src/components/Button.js"
    assert_output --partial "src/index.js"

    # Should NOT match project/tests/test.js
    refute_output --partial "tests/test.js"
}

@test "file contention detection with dir-scoped steps" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        steps {
            ["step1"] {
                dir = "src"
                glob = List("**/*.rs")
                check = "echo checking {{files}}"
                fix = "sh -c 'for f in {{files}}; do echo // step1 >> \$f; done'"
                check_first = true
            }
            ["step2"] {
                dir = "src"
                glob = List("lib.rs")
                check = "echo checking {{files}}"
                fix = "sh -c 'for f in {{files}}; do echo // step2 >> \$f; done'"
                check_first = true
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p src
    echo "fn main() {}" > src/lib.rs

    git add src/lib.rs

    run hk fix
    assert_success

    # Both steps should recognize contention on src/lib.rs
    # If contention is not detected properly, both will try to modify the file concurrently
    # With proper contention detection, step1 runs check first, then step2 runs check first

    # Verify both steps ran
    assert_output --partial "step1"
    assert_output --partial "step2"
}
