#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "staged_renamed_files detects renamed files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["check-renames"] {
                condition = """
git.staged_renamed_files != []
"""
                check = "echo 'Renamed files detected'"
            }
        }
    }
}
EOF
    git init
    git add hk.pkl
    git commit -m "initial commit"

    # Create a file and commit it
    echo "original content" > original.txt
    git add original.txt
    git commit -m "add original file"

    # Rename the file using git mv
    git mv original.txt renamed.txt

    # The hook should detect the rename
    run hk run pre-commit
    assert_success
    assert_output --partial "Renamed files detected"
}

@test "staged_renamed_files empty when no renames" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["check-renames"] {
                condition = """
git.staged_renamed_files != []
"""
                check = "echo 'Renamed files detected'"
            }
            ["check-no-renames"] {
                condition = """
git.staged_renamed_files == []
"""
                check = "echo 'No renames detected'"
            }
        }
    }
}
EOF
    git init
    git add hk.pkl
    git commit -m "initial commit"

    # Create a new file without renaming
    echo "new content" > new.txt
    git add new.txt

    # The hook should NOT detect renames
    run hk run pre-commit
    assert_success
    assert_output --partial "No renames detected"
    refute_output --partial "Renamed files detected"
}

@test "staged_renamed_files detects multiple renames" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["check-renames"] {
                condition = """
git.staged_renamed_files != []
"""
                check = "echo 'Renamed files detected'"
            }
        }
    }
}
EOF
    git init
    git add hk.pkl
    git commit -m "initial commit"

    # Create multiple files and commit them
    echo "content1" > file1.txt
    echo "content2" > file2.txt
    git add file1.txt file2.txt
    git commit -m "add files"

    # Rename both files using git mv
    git mv file1.txt renamed1.txt
    git mv file2.txt renamed2.txt

    # The hook should detect both renames
    run hk run pre-commit
    assert_success
    assert_output --partial "Renamed files detected"
}
