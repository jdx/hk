#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup

    # Set up a custom HK_STATE_DIR for isolated testing
    export HK_STATE_DIR="$BATS_TEST_TMPDIR/hk_state"
    mkdir -p "$HK_STATE_DIR/patches"
}

teardown() {
    _common_teardown
}

@test "patch backup created when stashing" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["trailing-whitespace"] = Builtins.trailing_whitespace
        }
    }
}
PKL
    git add hk.pkl
    git commit -m "init"

    # Create a file with staged and unstaged changes
    echo "staged content  " > file.txt
    git add file.txt
    echo "unstaged content  " > file.txt

    # Run pre-commit hook which should stash and create patch backup
    run hk run pre-commit
    assert_success

    # Verify patch file was created
    run bash -c "ls -1 $HK_STATE_DIR/patches/*.patch | wc -l | xargs"
    assert_success
    assert_output "1"

    # Verify patch file contains the unstaged changes
    run bash -c "cat $HK_STATE_DIR/patches/*.patch"
    assert_success
    assert_output --partial "unstaged content"

    # Verify patch filename format: {repo}-{timestamp}-{hash}.patch
    run bash -c "ls $HK_STATE_DIR/patches/*.patch"
    assert_success
    assert_output --regexp ".*-[0-9]{8}-[0-9]{6}-[a-f0-9]{8}\.patch"
}


@test "patch backup rotation respects configured limit" {
    # Override default (20) with test value (10) via environment variable
    export HK_STASH_BACKUP_COUNT=10

    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["test-step"] {
                check = "echo 'checking'"
            }
        }
    }
}
PKL
    git add hk.pkl
    git commit -m "init"

    # Create and commit initial file
    echo "initial" > file.txt
    git add file.txt
    git commit -m "add file"

    # Create 12 stashes to test rotation
    for i in {1..12}; do
        echo "content-$i" > file.txt
        git add file.txt
        echo "unstaged-$i" > file.txt
        hk run pre-commit > /dev/null 2>&1
        sleep 0.2  # Ensure different timestamps
    done

    # Verify exactly 10 patches remain (as configured)
    run bash -c "ls -1 $HK_STATE_DIR/patches/*.patch | wc -l | xargs"
    assert_success
    assert_output "10"

    # Get list of patch files sorted by time (newest first)
    run bash -c "ls -t $HK_STATE_DIR/patches/*.patch"
    assert_success

    # Verify we have patches (count non-empty lines)
    run bash -c "ls -t $HK_STATE_DIR/patches/*.patch | wc -l | xargs"
    assert_output "10"

    # Verify the most recent patch exists and contains recent content
    run bash -c "cat \$(ls -t $HK_STATE_DIR/patches/*.patch | head -1)"
    assert_success
    assert_output --partial "unstaged-12"
}

@test "patch backup is per-repository" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["test-step"] {
                check = "echo 'checking'"
            }
        }
    }
}
PKL
    git add hk.pkl
    git commit -m "init"

    # Create a stash in this repo
    echo "staged" > file.txt
    git add file.txt
    echo "unstaged" > file.txt
    run hk run pre-commit
    assert_success

    # Get the current repo name from patch filename
    run bash -c "ls $HK_STATE_DIR/patches/*.patch | head -1 | xargs basename"
    assert_success
    local patch_name="$output"

    # Verify patch filename starts with repo directory name
    # The repo is in a temp dir created by bats, extract the basename
    local repo_name=$(basename "$PWD")
    assert_output --partial "$repo_name"
}

@test "patch backup works with libgit2" {
    export HK_LIBGIT2=1

    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["trailing-whitespace"] = Builtins.trailing_whitespace
        }
    }
}
PKL
    git add hk.pkl
    git commit -m "init"

    # Create a file with staged and unstaged changes
    echo "staged content  " > file.txt
    git add file.txt
    echo "unstaged content  " > file.txt

    # Run pre-commit hook with libgit2
    run hk run pre-commit
    assert_success

    # Debug: check if patches directory exists and what's in it
    run bash -c "ls -la $HK_STATE_DIR/patches/ 2>&1 || echo 'patches dir does not exist'"
    echo "# Debug patches dir: $output" >&3

    # Verify patch file was created
    run bash -c "ls -1 $HK_STATE_DIR/patches/*.patch 2>/dev/null | wc -l | xargs"
    assert_success
    assert_output "1"

    # Verify patch content
    run bash -c "cat $HK_STATE_DIR/patches/*.patch"
    assert_success
    assert_output --partial "unstaged content"
}

@test "patch backup contains recoverable changes" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["test-step"] {
                check = "echo 'checking'"
            }
        }
    }
}
PKL
    git add hk.pkl
    git commit -m "init"

    # Create and commit initial file
    echo "original content" > file.txt
    git add file.txt
    git commit -m "add file"

    # Stage a change
    echo "staged change" > file.txt
    git add file.txt

    # Make unstaged change (this will be in the patch)
    echo "my important unstaged changes" > file.txt

    # Run pre-commit hook
    run hk run pre-commit
    assert_success

    # Verify patch was created
    run bash -c "ls -1 $HK_STATE_DIR/patches/*.patch | wc -l"
    assert_output "       1"

    # Verify patch contains the unstaged changes
    run bash -c "cat $HK_STATE_DIR/patches/*.patch"
    assert_success
    assert_output --partial "my important unstaged changes"

    # Verify patch is a valid unified diff format
    run bash -c "head -1 $HK_STATE_DIR/patches/*.patch"
    assert_output --partial "diff --git"
}
