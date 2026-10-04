#!/usr/bin/env bats

# While hk has the unstaged changes stashed, a journal in the git directory
# says so. A terminated hk puts the changes back and removes it; a killed one
# leaves it for the next run to recover from.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

# A pre-commit hook whose step signals that it started, then waits.
write_config() {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["slow"] {
        glob = "**/*.txt"
        fix = "touch started; sleep 30"
      }
    }
  }
  ["check"] {
    steps {
      ["quick"] {
        glob = "**/*.txt"
        check = "true"
      }
    }
  }
}
PKL
}

prepare_repo() {
    printf 'base\n' > file.txt
    git add hk.pkl file.txt
    git commit -m "init"
    printf 'staged\n' > file.txt
    git add file.txt
    printf 'staged\nunstaged\n' > file.txt
    JOURNAL="$(git rev-parse --absolute-git-dir)/hk-pending-stash"
}

wait_for_step() {
    for _ in $(seq 100); do
        [ -e started ] && return 0
        sleep 0.1
    done
    return 1
}

assert_changes_restored() {
    assert_equal "$(cat file.txt)" "$(printf 'staged\nunstaged\n')"
    assert_equal "$(git show :file.txt)" "staged"
    assert_equal "$(git stash list | wc -l | tr -d ' ')" "0"
    [ ! -e "$JOURNAL" ]
}

term_mid_step() {
    local use_libgit2="$1" sig="$2" want="$3"
    write_config
    prepare_repo
    HK_LIBGIT2="$use_libgit2" hk run pre-commit >/dev/null 2>&1 &
    local pid=$!
    wait_for_step
    # While the step runs, the changes are stashed and the journal says so
    assert_equal "$(cat file.txt)" "staged"
    assert_file_exists "$JOURNAL"
    run grep -c '"commit"' "$JOURNAL"
    assert_output 1
    kill "-$sig" "$pid"
    local status=0
    wait "$pid" || status=$?
    assert_equal "$status" "$want"
    assert_changes_restored
    # Nothing was left running
    run pgrep -f "sleep 30"
    assert_failure
}

@test "SIGTERM mid-step restores the stash and removes the journal (libgit2)" {
    term_mid_step 1 TERM 143
}

@test "SIGTERM mid-step restores the stash and removes the journal (shell git)" {
    term_mid_step 0 TERM 143
}

@test "SIGHUP mid-step restores the stash even when output is gone (libgit2)" {
    term_mid_step 1 HUP 129
}

@test "SIGINT mid-step restores the stash and exits 130 (shell git)" {
    term_mid_step 0 INT 130
}

kill9_then_recover() {
    local use_libgit2="$1"
    write_config
    prepare_repo
    HK_LIBGIT2="$use_libgit2" hk run pre-commit >/dev/null 2>&1 &
    local pid=$!
    wait_for_step
    kill -9 "$pid"
    wait "$pid" || true
    pkill -f "sleep 30" || true
    rm -f started
    # Killed: the changes are stranded in the stash, and the journal names them
    assert_equal "$(cat file.txt)" "staged"
    assert_file_exists "$JOURNAL"
    commit="$(git stash list --format=%H)"
    run grep "$commit" "$JOURNAL"
    assert_success
}

@test "after SIGKILL the next run restores a clean worktree's stash (libgit2)" {
    kill9_then_recover 1
    # Any run recovers, even one that stashes nothing
    run env HK_LIBGIT2=1 hk check --all
    assert_changes_restored
}

@test "after SIGKILL the next run restores a clean worktree's stash (shell git)" {
    kill9_then_recover 0
    run env HK_LIBGIT2=0 hk check --all
    assert_changes_restored
}

@test "after SIGKILL a dirty worktree is reported with the exact command and nothing is dropped" {
    kill9_then_recover 1
    printf 'something else\n' > new.txt
    commit="$(git stash list --format=%H)"
    run env HK_LIBGIT2=1 hk check --all
    assert_output --partial "git stash apply $commit"
    assert_file_exists "$JOURNAL"
    assert_equal "$(git stash list --format=%H)" "$commit"
    assert_equal "$(cat file.txt)" "staged"
    # Once the user has applied and dropped the entry, hk stops mentioning it
    rm new.txt
    assert_output --partial "git restore --source=<commit>"
    git restore --source="$commit" --worktree -- file.txt
    git stash drop
    run env HK_LIBGIT2=1 hk check --all
    refute_output --partial "git stash apply"
    [ ! -e "$JOURNAL" ]
}

@test "two runs recovering the same dead journal restore the stash once and leave no temporary files" {
    kill9_then_recover 1
    env HK_LIBGIT2=1 hk check --all >/dev/null 2>&1 &
    local a=$!
    env HK_LIBGIT2=0 hk check --all >/dev/null 2>&1 &
    local b=$!
    wait "$a" || true
    wait "$b" || true
    assert_changes_restored
    run bash -c 'ls "$(git rev-parse --absolute-git-dir)" | grep "hk-pending-stash" | grep -vc "^hk-pending-stash.lock$"'
    assert_output 0
}

@test "a journal whose process is alive is never touched" {
    write_config
    prepare_repo
    HK_LIBGIT2=1 hk run pre-commit >/dev/null 2>&1 &
    local pid=$!
    wait_for_step
    before="$(cat "$JOURNAL")"
    # A second hk in the same worktree leaves the running one's journal alone
    run env HK_LIBGIT2=1 hk check --all
    assert_equal "$(cat "$JOURNAL")" "$before"
    kill -TERM "$pid"
    wait "$pid" || true
    assert_changes_restored
}

# hk dies right after creating its stash entry, before the journal records it.
# The entry's message carries hk's pid, so the next run still finds it.
crash_before_record() {
    local use_libgit2="$1"
    write_config
    prepare_repo
    run env HK_DEBUG_KILL_BEFORE_JOURNAL_RECORD=1 HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_failure
    # The changes are stranded in the stash and the journal names nothing
    assert_equal "$(cat file.txt)" "staged"
    assert_file_exists "$JOURNAL"
    run grep -c '"commit"' "$JOURNAL"
    assert_output 0
    commit="$(git stash list --format=%H)"
    [ -n "$commit" ]
    run git stash list --format=%gs
    assert_output --regexp "hk: [0-9]+-"
    run env HK_LIBGIT2="$use_libgit2" hk check --all
    assert_output --partial "git stash apply $commit"
    assert_file_exists "$JOURNAL"
    assert_equal "$(git stash list --format=%H)" "$commit"
}

@test "a stash made just before hk is killed is reported by the next run (libgit2)" {
    crash_before_record 1
}

@test "a stash made just before hk is killed is reported by the next run (shell git)" {
    crash_before_record 0
}

# A recovering run that is killed leaves the journal where it was: nothing was
# renamed or claimed, so the next run finds it at the well-known path.
kill_during_recovery() {
    local use_libgit2="$1" point="$2"
    kill9_then_recover "$use_libgit2"
    run env "$point=1" HK_LIBGIT2="$use_libgit2" hk check --all
    assert_failure
    assert_file_exists "$JOURNAL"
    run bash -c 'ls "$(git rev-parse --absolute-git-dir)" | grep "hk-pending-stash" | grep -vc "^hk-pending-stash.lock$"'
    assert_output 1
    run env HK_LIBGIT2="$use_libgit2" hk check --all
    assert_changes_restored
}

@test "a recovery killed before it restores leaves the journal for the next run (libgit2)" {
    kill_during_recovery 1 HK_DEBUG_KILL_BEFORE_RECOVERY
}

@test "a recovery killed after it restores leaves the journal for the next run (shell git)" {
    kill_during_recovery 0 HK_DEBUG_KILL_AFTER_RECOVERY
}
