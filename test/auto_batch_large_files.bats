#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "auto-batch large file lists" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["count-files"] {
                check = "echo 'Processing {{files}}' | wc -w"
            }
        }
    }
}
EOF

    # Create a large number of files with long paths to exceed ARG_MAX safe limit
    # We'll create files in deeply nested directories to ensure long paths
    for i in {1..1000}; do
        dir="very/long/directory/path/number/$i/with/many/levels/to/increase/path/length"
        mkdir -p "$dir"
        echo "test" > "$dir/file_with_very_long_name_to_increase_arg_size_$i.txt"
    done

    # Run check and verify it completes successfully without "Argument list too long" errors
    run hk check
    assert_success

    # The output should show multiple batches were created
    # Each batch should process a subset of files
    # We verify by checking that the command executed without errors
}

@test "auto-batch does not break small file lists" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["echo-files"] {
                check = "echo {{files}}"
            }
        }
    }
}
EOF

    # Create a small number of files
    for i in {1..10}; do
        echo "test" > "file$i.txt"
    done

    run hk check
    assert_success

    # All files should be passed in a single command (no batching needed)
    assert_output --partial "file1.txt"
    assert_output --partial "file10.txt"
}

@test "auto-batch with fix command" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["count"] {
                fix = "echo 'Fixing {{files}}'"
            }
        }
    }
}
EOF

    # Create many files
    for i in {1..100}; do
        echo "test" > "file$i.txt"
    done

    run hk fix
    assert_success
    assert_output --partial "Fixing"
}

