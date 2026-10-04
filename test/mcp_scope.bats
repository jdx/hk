#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

write_capture_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["capture"] {
                check = "printf '%s\\n' {{files}} | sort > seen.txt"
            }
        }
    }
}
EOF
}

# One file with an edit in the index, one with an edit only in the worktree.
stage_one_edit() {
    write_capture_config
    echo one > staged.txt
    echo one > unstaged.txt
    git add .
    git commit -m init

    echo two > staged.txt
    echo two > unstaged.txt
    git add staged.txt
}

@test "mcp staged scope checks staged files and ignores unstaged edits" {
    stage_one_edit
    echo untracked > untracked.txt

    run python3 "$PROJECT_ROOT/test/test_helper/mcp_scope.py" staged
    assert_success
    assert_output "status=succeeded"

    run cat seen.txt
    assert_output "staged.txt"
}

@test "mcp unstaged scope still skips staged files" {
    stage_one_edit

    run python3 "$PROJECT_ROOT/test/test_helper/mcp_scope.py" unstaged
    assert_success
    assert_output "status=succeeded"

    run cat seen.txt
    assert_output "unstaged.txt"
}
