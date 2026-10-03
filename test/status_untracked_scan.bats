#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# `git status` walks the whole worktree to find untracked files, so hk only
# asks for them when the run can use them. GIT_TRACE records the status
# command hk ran.
write_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] { glob = "*.txt"; check = "echo checked {{files}} > checked.out" $1 }
        }
    }
}
EOF
    git add hk.pkl
    git commit -q -m "initial commit"
    echo tracked > a.txt
    git add a.txt
    git commit -q -m "add a"
    echo untracked > b.txt
}

untracked_mode() {
    if ! GIT_TRACE="$TEST_TEMP_DIR/trace.log" hk check "$@" > "$TEST_TEMP_DIR/out.log" 2>&1; then
        cat "$TEST_TEMP_DIR/out.log"
        return 1
    fi
    grep -o -e '--untracked-files=[a-z]*' "$TEST_TEMP_DIR/trace.log" | head -1
}

@test "hk check FILE does not look for untracked files" {
    write_config ""
    run untracked_mode a.txt
    [ "$status" -eq 0 ]
    [ "${lines[-1]}" = "--untracked-files=no" ]
}

@test "hk check FILE still runs on an untracked file" {
    write_config ""
    run untracked_mode b.txt
    [ "$status" -eq 0 ]
    [ "${lines[-1]}" = "--untracked-files=no" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check --all looks for untracked files and checks them" {
    write_config ""
    run untracked_mode --all
    [ "$status" -eq 0 ]
    [ "${lines[-1]}" = "--untracked-files=all" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check --glob looks for untracked files and checks them" {
    write_config ""
    run untracked_mode --glob '*.txt'
    [ "$status" -eq 0 ]
    [ "${lines[-1]}" = "--untracked-files=all" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check with no files looks for untracked files" {
    write_config ""
    run untracked_mode
    [ "$status" -eq 0 ]
    [ "${lines[-1]}" = "--untracked-files=all" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check FILE looks for untracked files when a condition reads them" {
    write_config '; condition = "git.untracked_files != []"'
    run untracked_mode a.txt
    [ "$status" -eq 0 ]
    [ "${lines[-1]}" = "--untracked-files=all" ]
    assert_file_exists checked.out
}

@test "hk check FILE looks for untracked files when stashing" {
    write_config ""
    GIT_TRACE="$TEST_TEMP_DIR/trace.log" run hk check --stash=git a.txt
    echo "$output"
    [ "$status" -eq 0 ]
    run grep -o -e '--untracked-files=[a-z]*' "$TEST_TEMP_DIR/trace.log"
    assert_line --index 0 "--untracked-files=all"
}
