#!/usr/bin/env bats

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

@test "files_between_refs uses merge base correctly" {
    # Create initial commit
    echo "base content" > base.txt
    git add base.txt
    git commit -m "Initial commit"
    BASE_COMMIT=$(git rev-parse HEAD)

    # Create and switch to feature branch
    git checkout -b feature
    echo "feature content" > feature.txt
    git add feature.txt
    git commit -m "Add feature.txt"
    FEATURE_COMMIT=$(git rev-parse HEAD)

    # Switch back to main and make changes
    git checkout main
    echo "main content" > main.txt
    git add main.txt
    git commit -m "Add main.txt"
    MAIN_COMMIT=$(git rev-parse HEAD)

    # Merge feature branch into main
    git merge feature -m "Merge feature branch"
    MERGE_COMMIT=$(git rev-parse HEAD)

    # Create hk.pkl with a simple step that just prints the files
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        steps {
            ["print-files"] {
                check = "echo '{{files}}'"
            }
        }
    }
}
EOF

    # Test files between base and feature
    run hk fix --from-ref=$BASE_COMMIT --to-ref=$FEATURE_COMMIT
    assert_success
    assert_output --partial "print-files – 1 file –  – echo 'feature.txt'"
    assert_output --partial "print-files – feature.txt"

    # Test files between base and merge commit
    run hk fix --from-ref=$BASE_COMMIT --to-ref=$MERGE_COMMIT
    assert_success
    assert_output --partial "print-files – 2 files –  – echo 'feature.txt main.txt'"
    assert_output --partial "print-files – feature.txt main.txt"

    # Test files between feature and merge commit
    run hk fix --from-ref=$FEATURE_COMMIT --to-ref=$MERGE_COMMIT
    assert_success
    assert_output --partial "print-files – 1 file –  – echo 'main.txt'"
    assert_output --partial "print-files – main.txt"
}

@test "files staged for deletion are not included with --all" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
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

    # Stage the file for deletion
    git rm src/foo.rs
    
    # Try to commit with --all
    run hk check --all
    assert_success
    assert_output --partial "foo: hk.pkl"
    # Verify the deleted file is not in the output
    refute_output --partial "src/foo.rs"
}
