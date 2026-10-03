#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# Two linked worktrees share one stash stack and one lock file in the common
# git directory. Each runs a hook whose step records when it starts and ends;
# the step only runs while that worktree's unstaged changes are stashed, so
# overlapping records mean overlapping stash phases.
_setup_two_worktrees() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["record"] {
        glob = "*.txt"
        check = "echo \"begin \$(basename \"\$PWD\")\" >> \"\$HK_TEST_LOG\"; sleep 1; echo \"end \$(basename \"\$PWD\")\" >> \"\$HK_TEST_LOG\""
      }
    }
  }
}
PKL
    echo base > file.txt
    git add hk.pkl file.txt
    git commit -m init
    WT_DIR="$TEST_TEMP_DIR/wts"; mkdir -p "$WT_DIR"
    git worktree add -q "$WT_DIR/wt-a" -b wt-a
    git worktree add -q "$WT_DIR/wt-b" -b wt-b
    for d in wt-a wt-b; do
        echo "staged $d" > "$WT_DIR/$d/file.txt"
        git -C "$WT_DIR/$d" add file.txt
        echo "unstaged $d" > "$WT_DIR/$d/file.txt"
    done
    export HK_TEST_LOG="$WT_DIR/log"
    : > "$HK_TEST_LOG"
}

@test "stash phases of hooks in two linked worktrees do not overlap" {
    _setup_two_worktrees

    (cd "$WT_DIR/wt-a" && hk run pre-commit >"$WT_DIR/a.out" 2>&1) &
    pid_a=$!
    (cd "$WT_DIR/wt-b" && hk run pre-commit >"$WT_DIR/b.out" 2>&1) &
    pid_b=$!
    wait "$pid_a"
    wait "$pid_b"

    run cat "$HK_TEST_LOG"
    assert_success
    # Each begin is followed by its own end before the other begins.
    [ "${#lines[@]}" -eq 4 ]
    [[ "${lines[0]}" == begin\ * ]]
    [ "${lines[1]}" = "end ${lines[0]#begin }" ]
    [[ "${lines[2]}" == begin\ * ]]
    [ "${lines[3]}" = "end ${lines[2]#begin }" ]
    [ "${lines[0]}" != "${lines[2]}" ]

    # Both restored their own unstaged edit and left no stash entries behind.
    run cat "$WT_DIR/wt-a/file.txt"
    assert_output "unstaged wt-a"
    run cat "$WT_DIR/wt-b/file.txt"
    assert_output "unstaged wt-b"
    run git stash list
    assert_output ""
    # The one that waited said so.
    run cat "$WT_DIR/a.out" "$WT_DIR/b.out"
    assert_output --partial "waiting for another hk process to finish stashing"
    assert_output --partial "hk-stash.lock"
}

@test "stash lock wait times out with an error naming the lock file" {
    _setup_two_worktrees

    # Hold the lock the way another hk process would.
    lock="$(git rev-parse --path-format=absolute --git-common-dir)/hk-stash.lock"
    python3 - "$lock" <<'PY' &
import fcntl, sys, time
f = open(sys.argv[1], "w")
fcntl.flock(f, fcntl.LOCK_EX)
time.sleep(30)
PY
    holder=$!
    sleep 1

    cd "$WT_DIR/wt-a"
    HK_STASH_LOCK_TIMEOUT=1 run hk run pre-commit
    kill "$holder"
    wait "$holder" 2>/dev/null || true
    assert_failure
    assert_output --partial "timed out after 1s"
    assert_output --partial "$lock"
    # Nothing was stashed, so the worktree edit is intact.
    run cat file.txt
    assert_output "unstaged wt-a"
}
