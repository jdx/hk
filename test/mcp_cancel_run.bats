#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    # Never leave the long sleeps behind if a test fails.
    [ -z "${TOKEN:-}" ] || pkill -f "$TOKEN" 2>/dev/null || true
    _common_teardown
}

# `timeout` is not on macOS; perl's alarm kills a hang with SIGALRM (status 142).
_timeout() {
    perl -e 'alarm shift; exec @ARGV' "$@"
}

write_slow_config() {
    TOKEN="hkcancel$$x$RANDOM"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["slow"] {
                check = "touch started; sleep 300 & sleep 301; : $TOKEN"
            }
        }
    }
}
EOF
    git init .
    git add hk.pkl
    git commit -m init
}

@test "mcp cancel_run stops the real hk and its steps" {
    write_slow_config
    run _timeout 60 python3 "$PROJECT_ROOT/test/test_helper/mcp_cancel.py" "$TOKEN"
    assert_success
    assert_output --partial "status=cancelled"
}

@test "--cd execs the real hk so signals reach it" {
    write_slow_config
    cd ..
    hk --cd proj check --all >/dev/null 2>&1 &
    pid=$!
    for _ in $(seq 100); do
        [ -e proj/started ] && break
        sleep 0.1
    done
    assert_file_exists proj/started
    # The pid we launched is the hk that runs the step, not a wrapper around it.
    run pgrep -P "$pid" -f "$TOKEN"
    assert_success
    kill -INT "$pid"
    wait "$pid" || true
    run pgrep -f "$TOKEN"
    assert_failure
}
