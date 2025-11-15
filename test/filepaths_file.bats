#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "files_path: creates temp file with filenames" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["test"] { glob = "*.txt"; make_filespath_file=true; check = "cat {{ filepaths_file }}" } } }
}
EOF

    echo "file1" > test1.txt
    echo "file2" > test2.txt
    echo "file3" > test3.txt

    git add -A

    run hk check
    echo "$output"

    assert_success
    # Should contain the filenames that were in the temp file
    assert_output --partial "test1.txt"
    assert_output --partial "test2.txt"
    assert_output --partial "test3.txt"
}

@test "files_path: works with commands that expect file list input" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["count"] { glob = "*.txt"; make_filespath_file=true; check = "wc -l < {{filepaths_file}}" } } }
}
EOF

    echo "content" > file1.txt
    echo "content" > file2.txt
    echo "content" > file3.txt

    git add -A

    run hk check
    echo "$output"

    assert_success
    # Should have 3 lines in the temp file (one per filename)
    assert_output --partial "3"
}

@test "files_path: respects dir setting" {
    mkdir -p subdir

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["test"] { dir = "subdir"; glob = "*.txt"; make_filespath_file=true; check = "cat {{filepaths_file}}" } } }
}
EOF

    echo "content" > subdir/file1.txt
    echo "content" > subdir/file2.txt

    git add -A

    run hk check
    echo "$output"

    assert_success
    # Paths should be relative to dir
    assert_output --partial "file1.txt"
    assert_output --partial "file2.txt"
    # Should NOT have the "subdir/" prefix
    refute_output --partial "subdir/file1.txt"
}

@test "files_path: temp file is cleaned up" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["test"] { glob = "*.txt"; make_filespath_file=true; check = "echo {{filepaths_file}} > /tmp/hk_test_path.txt" } } }
}
EOF

    echo "content" > test.txt

    git add -A

    run hk check
    echo "$output"

    assert_success

    # Read the temp file path that was used
    temp_path=$(cat /tmp/hk_test_path.txt | tr -d '[:space:]')

    # Wait a moment for cleanup
    sleep 0.5

    # Temp file should be gone
    [ ! -f "$temp_path" ]

    # Clean up our test file
    rm -f /tmp/hk_test_path.txt
}
