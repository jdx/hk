#!/usr/bin/env bats

# A user's Ctrl-C stops commands; a stopped command is cancelled, not failed.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    # A failed assertion can end a test while hk and its sleeper still run.
    if [ -n "${HK_PID:-}" ]; then
        kill -9 "$HK_PID" 2>/dev/null || true
        wait "$HK_PID" 2>/dev/null || true
    fi
    # Anchored on the test's unique duration, never a name-wide pkill.
    if [ -n "${SLEEPER:-}" ]; then
        pkill -f "sleep $SLEEPER\$" || true
    fi
    _common_teardown
}

# Start `hk "$@"` in the background writing out.jsonl and err.txt, wait until
# every marker file named in $WAIT_FOR exists, send SIGINT, and wait for hk to
# exit (failing the test rather than hanging the suite). Sets $status.
_interrupt() {
    local libgit2=$1
    shift
    rm -f $WAIT_FOR
    HK_LIBGIT2=$libgit2 hk "$@" >out.jsonl 2>err.txt &
    local pid=$!
    HK_PID=$pid
    for _ in $(seq 100); do
        local ready=1
        for f in $WAIT_FOR; do [ -e "$f" ] || ready=0; done
        [ "$ready" = 1 ] && break
        sleep 0.1
    done
    for f in $WAIT_FOR; do assert_file_exists "$f"; done
    # Optionally hold the interrupt until a step's failure is on record, so
    # the failure provably happened before the Ctrl-C.
    if [ -n "${AFTER_FAILED:-}" ]; then
        for _ in $(seq 100); do
            [ "$(_step_status "$AFTER_FAILED")" = failed ] && break
            sleep 0.1
        done
        assert_equal "$(_step_status "$AFTER_FAILED")" failed
    fi
    kill -INT "$pid"
    for _ in $(seq 100); do
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.1
    done
    if kill -0 "$pid" 2>/dev/null; then
        kill -9 "$pid"
        # Anchored on this test's unique duration, never a name-wide pkill.
        pkill -f "sleep $SLEEPER\$" || true
        fail "hk hung after Ctrl-C (HK_LIBGIT2=$libgit2)"
    fi
    status=0
    wait "$pid" || status=$?
    pkill -f "sleep $SLEEPER\$" || true
}

_step_status() {
    jq -r --arg n "$1" 'select(.event == "step_completed" and .data.name == $n) | .data.status' out.jsonl
}

_run_field() {
    jq -r "select(.event == \"run_completed\") | .data | $1" out.jsonl
}

@test "Ctrl-C reports the interrupted step as cancelled, not failed" {
    SLEEPER="30.$$$RANDOM"
    WAIT_FOR="started"
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["slow"] { check = "touch started && sleep $SLEEPER" }
    }
  }
}
EOF2
    echo test > test.txt
    git add hk.pkl test.txt
    git commit -m init

    for libgit2 in 1 0; do
        _interrupt $libgit2 --format jsonl check --all
        # The exit status is what it has always been for Ctrl-C.
        assert_equal "$status" 1
        run _step_status slow
        assert_output "cancelled"
        run _run_field .status
        assert_output "cancelled"
        run _run_field '[.steps[] | select(.status == "failed")] | length'
        assert_output "0"
        run cat err.txt
        assert_output --partial "command was cancelled"
    done
}

@test "a real failure still reports failed" {
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["bad"] { check = "exit 1" }
    }
  }
}
EOF2
    echo test > test.txt
    git add hk.pkl test.txt
    git commit -m init

    for libgit2 in 1 0; do
        HK_LIBGIT2=$libgit2 run bash -c "hk --format jsonl check --all 2>/dev/null >out.jsonl"
        assert_failure
        run _step_status bad
        assert_output "failed"
        run _run_field .status
        assert_output "failed"
    done
}

@test "Ctrl-C after a real failure keeps the failure and cancels only the interrupted step" {
    SLEEPER="30.$$$RANDOM"
    WAIT_FOR="slow-started bad-done"
    AFTER_FAILED=bad
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["bad"] { check = "touch bad-done; exit 1" }
      ["slow"] { check = "touch slow-started && sleep $SLEEPER" }
    }
  }
}
EOF2
    echo test > test.txt
    git add hk.pkl test.txt
    git commit -m init

    for libgit2 in 1 0; do
        _interrupt $libgit2 --format jsonl check --all --no-fail-fast
        assert_equal "$status" 1
        run _step_status bad
        assert_output "failed"
        run _step_status slow
        assert_output "cancelled"
        run _run_field .status
        assert_output "failed"
    done
}
