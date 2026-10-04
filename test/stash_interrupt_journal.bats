#!/usr/bin/env bats

# While hk has the unstaged changes stashed, a journal in the git directory
# says so. A terminated hk puts the changes back and removes it; a killed one
# leaves it for the next run to recover from.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    # A sleep duration no other test (or file) uses, so a process check matches
    # only this test's step. A bare "sleep 30" also matches the sleepers of
    # tests running concurrently under bats --jobs.
    SLEEP_MARK="30.$$$RANDOM$RANDOM"
}

teardown() {
    [ -z "${SLEEP_MARK:-}" ] || pkill -f "sleep $SLEEP_MARK\$" 2>/dev/null || true
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
        fix = "touch started; sleep $SLEEP_MARK"
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
    # Nothing was left running. The step's process group is killed before hk
    # exits, but the kernel reaps the dead processes asynchronously, so a
    # zombie can still show up for a moment (seen on macOS).
    for _ in $(seq 50); do
        pgrep -f "sleep $SLEEP_MARK\$" >/dev/null || return 0
        sleep 0.1
    done
    run pgrep -f "sleep $SLEEP_MARK\$"
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

@test "SIGINT mid-step restores the stash and exits as a cancelled run (shell git)" {
    term_mid_step 0 INT 1
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
    pkill -f "sleep $SLEEP_MARK\$" || true
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

# The git directory can be shared by hosts or PID namespaces that cannot see
# each other's processes, so a pid that is not running proves nothing unless
# the journal was written where hk runs now. Rewrite the recorded identity
# (or drop it, as an older hk's journal has none) to play another host.
set_owner_host() {
    python3 - "$JOURNAL" "$1" <<'PY'
import json, sys
path, value = sys.argv[1], sys.argv[2]
with open(path) as f:
    j = json.load(f)
if value == "-":
    j.pop("owner_host", None)
else:
    j["owner_host"] = value
with open(path, "w") as f:
    json.dump(j, f, indent=2)
PY
}

foreign_owner_is_reported() {
    local use_libgit2="$1" identity="$2"
    kill9_then_recover "$use_libgit2"
    local original
    original="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["owner_host"])' "$JOURNAL")"
    set_owner_host "$identity"
    local commit
    commit="$(git stash list --format=%H)"
    run env HK_LIBGIT2="$use_libgit2" hk check --all
    assert_output --partial "git stash apply $commit"
    assert_output --partial "cannot tell whether it is still running"
    # Nothing was restored or dropped
    assert_file_exists "$JOURNAL"
    assert_equal "$(git stash list --format=%H)" "$commit"
    assert_equal "$(cat file.txt)" "staged"
    # The same journal written where hk runs is recovered as before
    set_owner_host "$original"
    run env HK_LIBGIT2="$use_libgit2" hk check --all
    assert_changes_restored
}

@test "a journal from another host is reported, never restored (libgit2)" {
    foreign_owner_is_reported 1 "elsewhere.example|pid:[4026539999]"
}

@test "a journal from another PID namespace is reported, never restored (shell git)" {
    foreign_owner_is_reported 0 "$(hostname)|pid:[1]"
}

@test "a journal with no recorded host is reported when its pid is not running (libgit2)" {
    foreign_owner_is_reported 1 "-"
}

# The same crash, but the journal looks like another host's (or an older
# hk's, with no host recorded), so hk cannot judge its owner. The entry the
# journal never recorded is still named in the report, and nothing is touched.
foreign_owner_unrecorded_entry_is_reported() {
    local use_libgit2="$1" identity="$2"
    write_config
    prepare_repo
    run env HK_DEBUG_KILL_BEFORE_JOURNAL_RECORD=1 HK_LIBGIT2="$use_libgit2" hk run pre-commit
    assert_failure
    run grep -c '"commit"' "$JOURNAL"
    assert_output 0
    local commit before
    commit="$(git stash list --format=%H)"
    [ -n "$commit" ]
    set_owner_host "$identity"
    before="$(cat "$JOURNAL")"
    run env HK_LIBGIT2="$use_libgit2" hk check --all
    assert_output --partial "git stash apply $commit"
    assert_output --partial "cannot tell whether it is still running"
    assert_output --partial "not recorded in the journal"
    # Nothing was restored, dropped or rewritten
    assert_file_exists "$JOURNAL"
    assert_equal "$(cat "$JOURNAL")" "$before"
    assert_equal "$(git stash list --format=%H)" "$commit"
    assert_equal "$(cat file.txt)" "staged"
}

@test "an unrecorded stash entry is reported for a journal from another host (libgit2)" {
    foreign_owner_unrecorded_entry_is_reported 1 "elsewhere.example|pid:[4026539999]"
}

@test "an unrecorded stash entry is reported for a journal from another host (shell git)" {
    foreign_owner_unrecorded_entry_is_reported 0 "elsewhere.example|pid:[4026539999]"
}

@test "an unrecorded stash entry is reported for a journal with no recorded host (libgit2)" {
    foreign_owner_unrecorded_entry_is_reported 1 "-"
}

@test "an unrecorded stash entry is reported for a journal with no recorded host (shell git)" {
    foreign_owner_unrecorded_entry_is_reported 0 "-"
}

# Recovery takes the same stash lock stashing does, so it cannot interleave
# with another worktree's stash or pop.
recovery_waits_for_the_stash_lock() {
    local use_libgit2="$1"
    kill9_then_recover "$use_libgit2"
    local commit lock
    commit="$(git stash list --format=%H)"
    lock="$(git rev-parse --path-format=absolute --git-common-dir)/hk-stash.lock"
    python3 - "$lock" "$BATS_TEST_TMPDIR/lock-held" <<'PY' &
import fcntl, sys, time
f = open(sys.argv[1], "w")
fcntl.flock(f, fcntl.LOCK_EX)
open(sys.argv[2], "w").close()
time.sleep(60)
PY
    local holder=$!
    for _ in $(seq 100); do
        [ -e "$BATS_TEST_TMPDIR/lock-held" ] && break
        sleep 0.1
    done
    if [ ! -e "$BATS_TEST_TMPDIR/lock-held" ]; then
        kill "$holder" 2>/dev/null || true
        fail "lock holder never took the lock"
    fi
    HK_STASH_LOCK_TIMEOUT=1 HK_LIBGIT2="$use_libgit2" run hk check --all
    assert_output --partial "waiting for another hk process to finish stashing"
    assert_output --partial "left its journal"
    # Nothing was restored while the lock was held elsewhere
    assert_file_exists "$JOURNAL"
    assert_equal "$(git stash list --format=%H)" "$commit"
    assert_equal "$(cat file.txt)" "staged"
    kill "$holder"
    wait "$holder" 2>/dev/null || true
    # Once it is free the next run recovers
    run env HK_LIBGIT2="$use_libgit2" hk check --all
    assert_changes_restored
}

@test "recovery waits for the stash lock and restores nothing without it (libgit2)" {
    recovery_waits_for_the_stash_lock 1
}

@test "recovery waits for the stash lock and restores nothing without it (shell git)" {
    recovery_waits_for_the_stash_lock 0
}

# A journal that was only reported (the tree had edits of its own) stays. A
# run that then stashes those edits and starts its fixers holds the stash lock,
# so a third run cannot see a clean tree and restore the old stash over them.
reported_journal_is_not_restored_over_a_running_fixer() {
    local use_libgit2="$1"
    kill9_then_recover "$use_libgit2"
    local old
    old="$(git stash list --format=%H)"
    printf 'staged\nnew edit\n' > file.txt
    HK_LIBGIT2="$use_libgit2" hk run pre-commit >"$BATS_TEST_TMPDIR/b.out" 2>&1 &
    local pid=$!
    wait_for_step
    # The second run reported the old journal and kept it, the changes it
    # reported are still in the stash, and its own edit is the one stashed now
    assert_file_exists "$JOURNAL"
    assert_equal "$(cat file.txt)" "staged"
    run git stash list --format=%H
    assert_equal "${#lines[@]}" 2
    # A third run sees a clean tree but may not recover while that is so
    HK_STASH_LOCK_TIMEOUT=1 HK_LIBGIT2="$use_libgit2" run hk check --all
    assert_output --partial "left its journal"
    assert_equal "$(cat file.txt)" "staged"
    run git stash list --format=%H
    assert_equal "${#lines[@]}" 2
    assert_file_exists "$JOURNAL"
    kill -TERM "$pid"
    wait "$pid" || true
    # The running one put its own edit back and left the reported entry alone
    assert_equal "$(cat file.txt)" "$(printf 'staged\nnew edit\n')"
    assert_equal "$(git stash list --format=%H)" "$old"
    assert_file_exists "$JOURNAL"
    grep -q "pending-stash journal is still in place" "$BATS_TEST_TMPDIR/b.out"
}

@test "a reported journal is not restored over a running fixer (libgit2)" {
    reported_journal_is_not_restored_over_a_running_fixer 1
}

@test "a reported journal is not restored over a running fixer (shell git)" {
    reported_journal_is_not_restored_over_a_running_fixer 0
}

# A foreign journal that names no stash entry still in the stash protects
# nothing, and left in place it would keep every later run from writing a
# journal of its own: it is removed, and the next hook run journals again.
stale_foreign_journal_is_removed() {
    local use_libgit2="$1"
    kill9_then_recover "$use_libgit2"
    set_owner_host "elsewhere.example|pid:[4026539999]"
    # The entry was applied and dropped by hand
    git stash drop
    run env HK_LIBGIT2="$use_libgit2" hk check --all
    assert_output --partial "removed the stale pending-stash journal"
    [ ! -e "$JOURNAL" ]
    rm -f started
    git checkout -- file.txt 2>/dev/null || true
    printf 'staged\nunstaged\n' > file.txt
    HK_LIBGIT2="$use_libgit2" hk run pre-commit >/dev/null 2>&1 &
    local pid=$!
    wait_for_step
    assert_file_exists "$JOURNAL"
    kill -TERM "$pid"
    wait "$pid" || true
}

@test "a stale journal from another host is removed and the next run journals again (libgit2)" {
    stale_foreign_journal_is_removed 1
}

@test "a stale journal from another host is removed and the next run journals again (shell git)" {
    stale_foreign_journal_is_removed 0
}

# Octal permission bits of a file, portably: GNU stat takes -c, BSD stat -f.
file_mode() {
    stat -c %a "$1" 2>/dev/null || stat -f %Lp "$1"
}

# core.sharedRepository=group asks for files other accounts of the group can
# use, whatever the creator's umask, so a journal a killed hk left can be read
# and recovered by another member.
shared_repository_journal_is_group_readable() {
    local use_libgit2="$1"
    write_config
    prepare_repo
    git config core.sharedRepository group
    umask 077
    HK_LIBGIT2="$use_libgit2" hk run pre-commit >/dev/null 2>&1 &
    local pid=$!
    wait_for_step
    assert_file_exists "$JOURNAL"
    kill -9 "$pid"
    wait "$pid" || true
    pkill -f "sleep $SLEEP_MARK\$" || true
    assert_equal "$(file_mode "$JOURNAL")" "660"
}

@test "a journal in a group-shared repository is group-readable under umask 077 (libgit2)" {
    shared_repository_journal_is_group_readable 1
}

@test "a journal in a group-shared repository is group-readable under umask 077 (shell git)" {
    shared_repository_journal_is_group_readable 0
}
