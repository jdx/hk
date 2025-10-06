#!/usr/bin/env bats

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
