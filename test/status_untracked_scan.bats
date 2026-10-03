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
commit_files() {
    git add hk.pkl
    git commit -q -m "initial commit"
    echo tracked > a.txt
    git add a.txt
    git commit -q -m "add a"
    echo untracked > b.txt
}

write_config() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] { glob = "*.txt"; check = "echo checked {{files}} > checked.out" }
        }
    }
}
EOF
    commit_files
}

write_config_with_condition() {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] {
                glob = "*.txt"
                check = "echo checked {{files}} > checked.out"
                condition = "git.untracked_files != []"
            }
        }
    }
}
EOF
    commit_files
}

# Runs `hk check ARGS` and sets MODE to the --untracked-files value of the
# first status command it started.
run_check() {
    rm -f "$TEST_TEMP_DIR/trace.log"
    if ! GIT_TRACE="$TEST_TEMP_DIR/trace.log" hk check "$@" > "$TEST_TEMP_DIR/out.log" 2>&1; then
        cat "$TEST_TEMP_DIR/out.log"
        return 1
    fi
    MODE=$(grep -o -e '--untracked-files=[a-z]*' "$TEST_TEMP_DIR/trace.log" | head -1)
}

@test "hk check FILE does not look for untracked files" {
    write_config
    run_check a.txt
    [ "$MODE" = "--untracked-files=no" ]
}

@test "hk check FILE still runs on an untracked file" {
    write_config
    run_check b.txt
    [ "$MODE" = "--untracked-files=no" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check --files0-from does not look for untracked files" {
    write_config
    printf 'b.txt\0' > files0
    run_check --files0-from files0
    [ "$MODE" = "--untracked-files=no" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check --from-ref does not look for untracked files" {
    write_config
    run_check --from-ref HEAD~1
    [ "$MODE" = "--untracked-files=no" ]
    assert_file_contains checked.out "a.txt"
}

@test "hk check --staged does not look for untracked files" {
    write_config
    echo more >> a.txt
    git add a.txt
    run_check --staged
    [ "$MODE" = "--untracked-files=no" ]
    assert_file_contains checked.out "a.txt"
}

@test "hk check --all looks for untracked files and checks them" {
    write_config
    run_check --all
    [ "$MODE" = "--untracked-files=all" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check --glob looks for untracked files and checks them" {
    write_config
    run_check --glob '*.txt'
    [ "$MODE" = "--untracked-files=all" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check --unstaged looks for untracked files and checks them" {
    write_config
    run_check --unstaged
    [ "$MODE" = "--untracked-files=all" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check with no files looks for untracked files" {
    write_config
    run_check
    [ "$MODE" = "--untracked-files=all" ]
    assert_file_contains checked.out "b.txt"
}

@test "hk check FILE looks for untracked files when a condition reads them" {
    write_config_with_condition
    run_check a.txt
    [ "$MODE" = "--untracked-files=all" ]
    assert_file_exists checked.out
}

@test "hk check FILE looks for untracked files when stashing" {
    write_config
    run_check --stash=git a.txt
    [ "$MODE" = "--untracked-files=all" ]
}
