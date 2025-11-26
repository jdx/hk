#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "stage directive should work when git commit run from subdirectory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["create-owners"] {
                condition = "git.staged_added_files != []"
                fix = """
# Create owners.yml in the same directory as the new file
for file in \$(git diff --cached --name-only --diff-filter=A); do
    dir=\$(dirname "\$file")
    echo "\$(basename "\$file"): team_a" > "\$dir/owners.yml"
done
"""
                stage = "**/owners.yml"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "init"
    hk install

    # Create a subdirectory and a new file in it
    mkdir -p web/test_dir
    echo "test content" > web/test_dir/test_file.txt
    git add web/test_dir/test_file.txt

    # Change to subdirectory before running hook (simulates user running git commit from subdirectory)
    cd web/test_dir

    # Run the pre-commit hook from the subdirectory
    hk run pre-commit

    # Go back to root to check results
    cd ../..

    # Verify that test_file.txt is staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "web/test_dir/test_file.txt"

    # Verify that owners.yml IS staged (this is the bug - it should be staged but isn't)
    assert_output --partial "web/test_dir/owners.yml"

    # Verify owners.yml exists
    run test -f web/test_dir/owners.yml
    assert_success

    # Verify owners.yml has correct content
    run cat web/test_dir/owners.yml
    assert_success
    assert_output "test_file.txt: team_a"
}

@test "stage directive works when git commit run from repo root" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["create-owners"] {
                condition = "git.staged_added_files != []"
                fix = """
# Create owners.yml in the same directory as the new file
for file in \$(git diff --cached --name-only --diff-filter=A); do
    dir=\$(dirname "\$file")
    echo "\$(basename "\$file"): team_a" > "\$dir/owners.yml"
done
"""
                stage = "**/owners.yml"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "init"
    hk install

    # Create a subdirectory and a new file in it
    mkdir -p web/test_dir
    echo "test content" > web/test_dir/test_file.txt
    git add web/test_dir/test_file.txt

    # Run the pre-commit hook from repo root (this should work)
    hk run pre-commit

    # Verify that test_file.txt is staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "web/test_dir/test_file.txt"

    # Verify that owners.yml IS staged
    assert_output --partial "web/test_dir/owners.yml"

    # Verify owners.yml exists
    run test -f web/test_dir/owners.yml
    assert_success

    # Verify owners.yml has correct content
    run cat web/test_dir/owners.yml
    assert_success
    assert_output "test_file.txt: team_a"
}

@test "stage directive stages modified files when git commit run from subdirectory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["update-owners"] {
                condition = "git.staged_added_files != []"
                fix = """
# Create or update owners.yml in the same directory as the new file
for file in \$(git diff --cached --name-only --diff-filter=A); do
    dir=\$(dirname "\$file")
    if [ -f "\$dir/owners.yml" ]; then
        # Append to existing file
        echo "\$(basename "\$file"): team_b" >> "\$dir/owners.yml"
    else
        # Create new file
        echo "\$(basename "\$file"): team_b" > "\$dir/owners.yml"
    fi
done
"""
                stage = "**/owners.yml"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "init"
    hk install

    # Create a subdirectory with an existing owners.yml file
    mkdir -p web/test_dir
    echo "existing_file.txt: team_a" > web/test_dir/owners.yml
    git add web/test_dir/owners.yml
    git commit -m "add existing owners.yml"

    # Create a new file in the same directory
    echo "test content" > web/test_dir/test_file.txt
    git add web/test_dir/test_file.txt

    # Change to subdirectory before running hook
    cd web/test_dir

    # Run the pre-commit hook from the subdirectory
    hk run pre-commit

    # Go back to root to check results
    cd ../..

    # Verify that test_file.txt is staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "web/test_dir/test_file.txt"

    # Verify that modified owners.yml IS staged
    assert_output --partial "web/test_dir/owners.yml"

    # Verify owners.yml has both entries
    run cat web/test_dir/owners.yml
    assert_success
    assert_output --partial "existing_file.txt: team_a"
    assert_output --partial "test_file.txt: team_b"
}

@test "stage directive stages files in sibling directories from subdirectory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["create-owners"] {
                condition = "git.staged_added_files != []"
                fix = """
# Create owners.yml in the same directory as the new file
for file in \$(git diff --cached --name-only --diff-filter=A); do
    dir=\$(dirname "\$file")
    echo "\$(basename "\$file"): team_a" > "\$dir/owners.yml"
done
"""
                stage = "**/owners.yml"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "init"
    hk install

    # Create sibling directories with new files in each
    mkdir -p web/dir1 web/dir2
    echo "content1" > web/dir1/file1.txt
    echo "content2" > web/dir2/file2.txt
    git add web/dir1/file1.txt web/dir2/file2.txt

    # Change to one subdirectory before running hook
    cd web/dir1

    # Run the pre-commit hook from subdirectory
    hk run pre-commit

    # Go back to root to check results
    cd ../..

    # Verify both original files are staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "web/dir1/file1.txt"
    assert_output --partial "web/dir2/file2.txt"

    # Verify owners.yml in BOTH directories are staged
    assert_output --partial "web/dir1/owners.yml"
    assert_output --partial "web/dir2/owners.yml"

    # Verify both owners.yml files exist and have correct content
    run test -f web/dir1/owners.yml
    assert_success
    run cat web/dir1/owners.yml
    assert_success
    assert_output "file1.txt: team_a"

    run test -f web/dir2/owners.yml
    assert_success
    run cat web/dir2/owners.yml
    assert_success
    assert_output "file2.txt: team_a"
}

@test "stage directive stages files in parent and child directories from subdirectory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["create-owners"] {
                condition = "git.staged_added_files != []"
                fix = """
# Create owners.yml in the same directory as the new file
for file in \$(git diff --cached --name-only --diff-filter=A); do
    dir=\$(dirname "\$file")
    echo "\$(basename "\$file"): team_a" > "\$dir/owners.yml"
done
"""
                stage = "**/owners.yml"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "init"
    hk install

    # Create parent and nested child directories with files
    mkdir -p web/parent/child
    echo "parent content" > web/parent/parent_file.txt
    echo "child content" > web/parent/child/child_file.txt
    git add web/parent/parent_file.txt web/parent/child/child_file.txt

    # Change to middle directory before running hook
    cd web/parent

    # Run the pre-commit hook from middle directory
    hk run pre-commit

    # Go back to root to check results
    cd ../..

    # Verify both original files are staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "web/parent/parent_file.txt"
    assert_output --partial "web/parent/child/child_file.txt"

    # Verify owners.yml in BOTH parent and child directories are staged
    assert_output --partial "web/parent/owners.yml"
    assert_output --partial "web/parent/child/owners.yml"

    # Verify both owners.yml files exist and have correct content
    run test -f web/parent/owners.yml
    assert_success
    run cat web/parent/owners.yml
    assert_success
    assert_output "parent_file.txt: team_a"

    run test -f web/parent/child/owners.yml
    assert_success
    run cat web/parent/child/owners.yml
    assert_success
    assert_output "child_file.txt: team_a"
}

@test "stage directive does not stage files not matching glob from subdirectory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["create-files"] {
                condition = "git.staged_added_files != []"
                fix = """
# Create both .yml and .md files in the same directory as the new file
for file in \$(git diff --cached --name-only --diff-filter=A); do
    dir=\$(dirname "\$file")
    echo "\$(basename "\$file"): team_a" > "\$dir/owners.yml"
    echo "# \$(basename "\$file")" > "\$dir/notes.md"
done
"""
                stage = "**/owners.yml"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "init"
    hk install

    # Create sibling directories with new files in each
    mkdir -p web/dir1 web/dir2
    echo "content1" > web/dir1/file1.txt
    echo "content2" > web/dir2/file2.txt
    git add web/dir1/file1.txt web/dir2/file2.txt

    # Change to one subdirectory before running hook
    cd web/dir1

    # Run the pre-commit hook from subdirectory
    hk run pre-commit

    # Go back to root to check results
    cd ../..

    # Verify original files are staged
    run git diff --name-only --cached
    assert_success
    assert_output --partial "web/dir1/file1.txt"
    assert_output --partial "web/dir2/file2.txt"

    # Verify owners.yml files ARE staged (they match the glob)
    assert_output --partial "web/dir1/owners.yml"
    assert_output --partial "web/dir2/owners.yml"

    # Verify .md files are NOT staged (they don't match the glob)
    refute_output --partial "web/dir1/notes.md"
    refute_output --partial "web/dir2/notes.md"

    # Verify .md files exist but remain untracked
    run test -f web/dir1/notes.md
    assert_success
    run test -f web/dir2/notes.md
    assert_success

    run git status --porcelain web/dir1/notes.md web/dir2/notes.md
    assert_success
    assert_line --regexp '^\?\? web/dir1/notes\.md$'
    assert_line --regexp '^\?\? web/dir2/notes\.md$'
}
