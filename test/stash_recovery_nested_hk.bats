#!/usr/bin/env bats

# Recovering a stash an interrupted hk left restores untracked files with
# `git restore`, which runs the post-checkout hook. That hook's hk must not
# wait for the stash lock its parent holds.

setup() {
    load 'test_helper/common_setup'
    _common_setup
    SLEEP_MARK="30.$$$RANDOM$RANDOM"
    MARK="$BATS_TEST_TMPDIR/started"
}

teardown() {
    [ -z "${SLEEP_MARK:-}" ] || pkill -f "sleep $SLEEP_MARK\$" 2>/dev/null || true
    _common_teardown
}

@test "recovery with untracked files does not start a nested hk that waits for the stash lock" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    stash = "git"
    steps {
      ["slow"] { check = "[ -e '$MARK' ] || { touch $MARK; sleep $SLEEP_MARK; }" }
    }
  }
  ["post-checkout"] {
    steps {
      ["noop"] { check = "true" }
    }
  }
}
PKL
    echo one > a.txt
    git add hk.pkl a.txt
    git commit -q -m base --no-verify
    hk install

    echo two >> a.txt
    git add a.txt
    echo "three (unstaged)" >> a.txt
    echo new > notes.txt

    hk run pre-commit >/dev/null 2>&1 &
    pid=$!
    for _ in $(seq 1 100); do
        [ -e "$MARK" ] && break
        sleep 0.1
    done
    [ -e "$MARK" ]
    kill -9 "$pid"
    wait "$pid" || true
    pkill -f "sleep $SLEEP_MARK\$" 2>/dev/null || true

    # A nested hk would wait the whole timeout for the lock
    export HK_STASH_LOCK_TIMEOUT=60
    start=$SECONDS
    run git commit -q -m two
    assert_success
    elapsed=$((SECONDS - start))
    echo "$output"
    [ "$elapsed" -lt 30 ]
    refute_output --partial "waiting for another hk process"
    assert_file_exists notes.txt
    assert_equal "$(cat notes.txt)" "new"
}
