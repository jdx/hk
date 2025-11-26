#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "broken symlinks are kept in file list for tools like check-symlinks" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["test"] {
                glob = "**/*"
                allow_symlinks = true
                check = "echo {{files}}"
            }
        }
    }
}
EOF

    # Create a regular file
    echo "content" > regular.txt
    git add regular.txt

    # Create a broken symlink (pointing to non-existent file)
    ln -s nonexistent.txt broken_link.txt
    git add broken_link.txt

    # Create a valid symlink to a file
    echo "target" > target.txt
    ln -s target.txt valid_link.txt
    git add target.txt valid_link.txt

    run hk check
    assert_success

    # All files including broken symlink should be in output
    assert_output --partial "regular.txt"
    assert_output --partial "broken_link.txt"
    assert_output --partial "valid_link.txt"
    assert_output --partial "target.txt"
}

@test "symlinks to directories are filtered from file list" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["test"] {
                glob = "**/*"
                check = "echo {{files}}"
            }
        }
    }
}
EOF

    # Create a directory and symlink to it
    mkdir -p somedir
    echo "content" > somedir/file.txt
    ln -s somedir dir_link
    git add somedir/file.txt dir_link

    # Create a regular file
    echo "content" > regular.txt
    git add regular.txt

    run hk check
    assert_success

    # Output should include regular.txt and somedir/file.txt
    assert_output --partial "regular.txt"
    assert_output --partial "somedir/file.txt"

    # Output should NOT include dir_link (symlink to directory)
    refute_output --partial "dir_link"
}

@test "regular files and valid symlinks are included" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["check"] {
        steps {
            ["test"] {
                glob = "**/*"
                allow_symlinks = true
                check = "echo {{files}}"
            }
        }
    }
}
EOF

    # Create a regular file
    echo "content" > regular.txt
    git add regular.txt

    # Create a valid symlink to a file
    echo "target" > target.txt
    ln -s target.txt valid_link.txt
    git add target.txt valid_link.txt

    run hk check
    assert_success

    # All files should be in the output
    assert_output --partial "regular.txt"
    assert_output --partial "target.txt"
    assert_output --partial "valid_link.txt"
}
