#!/usr/bin/env bats

# Test that directory traversal respects .gitignore
# See: https://github.com/jdx/hk/discussions/629

setup() {
    load 'test_helper/common_setup'
    _common_setup
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["log-files"] {
                glob = List("**/*.txt")
                check = "echo {{files}} > processed_files.log"
            }
        }
    }
}
EOF

}
teardown() {
    _common_teardown
}

@test "hk check . ignores .gitignore'd files" {
    echo "tracked" > tracked.txt
    mkdir -p ignored_dir
    echo "ignored" > ignored_dir/ignored.txt

    echo "ignored_dir/" > .gitignore

    run hk check .
    assert_success

    run cat processed_files.log
    assert_output "tracked.txt"
}

@test "hk check . ignores .git directory" {
    echo "test" > test.txt

    run hk check .
    assert_success

    run cat processed_files.log
    refute_output --partial ".git"
}
