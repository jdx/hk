#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    # Only this test's own sleeper (unique duration), never other tests' sleeps.
    [ -z "${SLEEPER:-}" ] || pkill -f "sleep $SLEEPER" 2>/dev/null || true
    [ -z "${STUB_SLEEPER:-}" ] || pkill -f "sleep $STUB_SLEEPER" 2>/dev/null || true
    _common_teardown
}

@test "fail_fast=true aborts on first failure" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
  ["check"] {
    steps {
      ["first"] {
        exclusive = true
        check = "sh -c 'echo FIRST && exit 2'"
      }
      ["second"] { check = "echo SECOND" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "test" > test.txt

    run hk check
    assert_failure
    assert_output --partial "FIRST"
    # Should not run the second step when fail_fast=true
    refute_output --partial "SECOND"
}

@test "fail_fast=false continues after failure" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
  ["check"] {
    steps {
      ["first"] {
        exclusive = true
        check = "sh -c 'echo FIRST && exit 2'"
      }
      ["second"] { check = "echo SECOND" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "test" > test.txt

    run hk check
    # Overall run still fails due to first step
    assert_failure
    assert_output --partial "FIRST"
    # With fail_fast=false, the second step should still run
    assert_output --partial "SECOND"
}

@test "fail-fast reports the failing step, not a sibling it cancelled" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = true
hooks {
  ["check"] {
    steps {
      ["sleeper"] { check = "sleep 30" }
      ["broken"] { check = "sh -c 'echo REAL-FAILURE >&2; exit 3'" }
    }
  }
}
EOF
    git add hk.pkl
    git commit -m "init"
    echo "test" > test.txt

    # The cancelled sibling must never win the race to be the reported error.
    for libgit2 in 1 0; do
        for i in $(seq 1 15); do
            start=$SECONDS
            HK_LIBGIT2=$libgit2 run hk check
            assert_failure
            assert [ $((SECONDS - start)) -lt 15 ]
            assert_output --partial "REAL-FAILURE"
            assert_output --partial "broken"
            refute_output --partial "cancelled"
        done
    done
}

@test "allow_failure does not hide a job that could not run in another workspace" {
    mkdir -p services/api/sub services/web
    touch services/api/module.toml services/web/module.toml
    echo a > services/api/a.mod
    echo b > services/web/b.mod
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["vet"] {
        glob = "**/*.mod"
        workspace_indicator = "module.toml"
        dir = "{{workspace}}/sub"
        allow_failure = true
        check = new Command { argv = List("sh", "-c", "echo ALLOWED-FAILURE >&2; exit 1") }
      }
    }
  }
}
EOF
    git add .
    git commit -m "init"
    for libgit2 in 1 0; do
        HK_LIBGIT2=$libgit2 run hk check --all
        assert_failure
        assert_output --partial "working directory does not exist"
    done
}

# Ctrl-C is a cancellation, not a step failure: fail-fast must not treat it as
# one, or the hook could report success (or hang on a dependent step).
@test "Ctrl-C under fail-fast exits as a cancelled hook, never as success" {
    # A duration unique to this test run, so cleanup can never match another
    # test's `sleep` (bats runs files in parallel).
    SLEEPER="30.$$$RANDOM"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["slow"] { check = "touch started && sleep $SLEEPER" }
      ["dependent"] { depends = "slow"; check = "echo DEPENDENT-RAN" }
    }
  }
}
EOF
    echo "test" > test.txt
    git add hk.pkl test.txt
    git commit -m "init"

    for libgit2 in 1 0; do
        rm -f started
        HK_LIBGIT2=$libgit2 hk check --all >out.txt 2>&1 &
        pid=$!
        for _ in $(seq 100); do
            [ -e started ] && break
            sleep 0.1
        done
        assert_file_exists started
        kill -INT "$pid"
        # Poll instead of `wait` so a hang fails the test rather than the suite.
        for _ in $(seq 100); do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.1
        done
        if kill -0 "$pid" 2>/dev/null; then
            kill -9 "$pid"
            pkill -f "sleep $SLEEPER" || true
            fail "hk hung after Ctrl-C (HK_LIBGIT2=$libgit2)"
        fi
        status=0
        wait "$pid" || status=$?
        # Same outcome as before fail-fast aborts existed: a failed hook that
        # reports the cancelled command.
        assert_equal "$status" 1
        run cat out.txt
        assert_output --partial "command was cancelled"
        refute_output --partial "DEPENDENT-RAN"
    done
}

# One workspace job is running its check while another waits on a `mise env`
# call that never returns. Ctrl-C must still end the run promptly instead of
# waiting for the stuck setup.
@test "Ctrl-C does not wait for a job stuck resolving the mise environment" {
    if ! command -v mise >/dev/null 2>&1; then
        skip "mise is not installed"
    fi
    # Durations unique to this test run, so cleanup never matches another
    # test's `sleep` (bats runs files in parallel).
    SLEEPER="30.$$$RANDOM"
    STUB_SLEEPER="60.$$$RANDOM"
    real_mise=$(command -v mise)
    mkdir -p stubbin
    cat <<EOF > stubbin/mise
#!/bin/sh
if [ "\$1" = env ]; then
    case "\$PWD" in
        */b) touch "$PWD/mise-stuck"; exec sleep $STUB_SLEEPER ;;
    esac
fi
exec "$real_mise" "\$@"
EOF
    chmod +x stubbin/mise
    export PATH="$PWD/stubbin:$PATH"
    export HK_MISE=1
    export MISE_TRUSTED_CONFIG_PATHS="$TEST_TEMP_DIR"
    mkdir -p a b
    touch a/module.toml b/module.toml
    echo a > a/f.mod
    echo b > b/f.mod
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["ws"] {
        glob = "**/*.mod"
        workspace_indicator = "module.toml"
        dir = "{{workspace}}"
        check = "touch \$PWD/../started && sleep $SLEEPER"
      }
    }
  }
}
EOF
    git add hk.pkl a b
    git commit -m "init"

    for libgit2 in 1 0; do
        rm -f started mise-stuck
        HK_LIBGIT2=$libgit2 hk check --all >out.txt 2>&1 &
        pid=$!
        for _ in $(seq 100); do
            [ -e started ] && [ -e mise-stuck ] && break
            sleep 0.1
        done
        assert_file_exists started
        assert_file_exists mise-stuck
        kill -INT "$pid"
        start=$SECONDS
        # Poll instead of `wait` so a hang fails the test rather than the suite.
        for _ in $(seq 200); do
            kill -0 "$pid" 2>/dev/null || break
            sleep 0.1
        done
        if kill -0 "$pid" 2>/dev/null; then
            kill -9 "$pid"
            pkill -f "sleep $SLEEPER" || true
            pkill -f "sleep $STUB_SLEEPER" || true
            fail "hk kept waiting after Ctrl-C (HK_LIBGIT2=$libgit2)"
        fi
        status=0
        wait "$pid" || status=$?
        # Within the 10s grace, and the same cancelled-hook outcome as a
        # Ctrl-C with no stuck job.
        assert [ $((SECONDS - start)) -lt 15 ]
        assert_equal "$status" 1
        run cat out.txt
        assert_output --partial "command was cancelled"
    done
}

# Sends Ctrl-C to `hk check --all "$@"` once `$1` exists and asserts the run
# ends within 15s with the status and message a Ctrl-C always had: a failed,
# cancelled hook, never success and never a hang. Dependent steps must not run.
_interrupt_and_expect_cancelled() {
    local marker=$1
    shift
    rm -f "$marker"
    hk check --all "$@" >out.txt 2>&1 &
    local pid=$!
    for _ in $(seq 100); do
        [ -e "$marker" ] && break
        sleep 0.1
    done
    assert_file_exists "$marker"
    sleep 0.3
    kill -INT "$pid"
    local start=$SECONDS
    # Poll instead of `wait` so a hang fails the test rather than the suite.
    for _ in $(seq 200); do
        kill -0 "$pid" 2>/dev/null || break
        sleep 0.1
    done
    if kill -0 "$pid" 2>/dev/null; then
        kill -9 "$pid"
        [ -z "${SLEEPER:-}" ] || pkill -f "sleep $SLEEPER" || true
        [ -z "${STUB_SLEEPER:-}" ] || pkill -f "sleep $STUB_SLEEPER" || true
        fail "hk hung after Ctrl-C"
    fi
    local status=0
    wait "$pid" || status=$?
    assert [ $((SECONDS - start)) -lt 15 ]
    assert_equal "$status" 1
    run cat out.txt
    assert_output --partial "command was cancelled"
    refute_output --partial "DEPENDENT-RAN"
}

# Every job is still resolving `mise env` when Ctrl-C arrives, so no command
# ever started. That is still a cancelled run (exit 1), not a successful one,
# and the step that depends on it must not hang or run.
@test "Ctrl-C while every job resolves the mise environment is a cancelled run" {
    if ! command -v mise >/dev/null 2>&1; then
        skip "mise is not installed"
    fi
    STUB_SLEEPER="60.$$$RANDOM"
    real_mise=$(command -v mise)
    mkdir -p stubbin
    cat <<EOF2 > stubbin/mise
#!/bin/sh
if [ "\$1" = env ]; then
    case "\$PWD" in
        */w) touch "$PWD/mise-stuck"; exec sleep $STUB_SLEEPER ;;
    esac
fi
exec "$real_mise" "\$@"
EOF2
    chmod +x stubbin/mise
    export PATH="$PWD/stubbin:$PATH"
    export HK_MISE=1
    export MISE_TRUSTED_CONFIG_PATHS="$TEST_TEMP_DIR"
    mkdir -p w
    touch w/module.toml
    echo w > w/f.mod
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["ws"] {
        glob = "**/*.mod"
        workspace_indicator = "module.toml"
        dir = "{{workspace}}"
        check = "echo RAN-CMD"
      }
      ["dependent"] { depends = "ws"; check = "echo DEPENDENT-RAN" }
    }
  }
}
EOF2
    git add hk.pkl w
    git commit -m "init"

    for libgit2 in 1 0; do
        for flags in "" "--no-fail-fast"; do
            export HK_LIBGIT2=$libgit2
            _interrupt_and_expect_cancelled mise-stuck $flags
        done
    done
    pkill -f "sleep $STUB_SLEEPER" || true
}

# Ctrl-C while a step waits for the step it depends on: the whole chain ends,
# none of the waiting steps runs, and nothing waits on a done notification that
# never comes.
@test "Ctrl-C with steps waiting on a dependency ends the whole chain" {
    SLEEPER="30.$$$RANDOM"
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["slow"] { check = "touch started && sleep $SLEEPER" }
      ["d1"] { depends = "slow"; check = "echo DEPENDENT-RAN" }
      ["d2"] { depends = "d1"; check = "echo DEPENDENT-RAN" }
    }
  }
}
EOF2
    echo "test" > test.txt
    git add hk.pkl test.txt
    git commit -m "init"

    for libgit2 in 1 0; do
        for flags in "" "--no-fail-fast"; do
            export HK_LIBGIT2=$libgit2
            _interrupt_and_expect_cancelled started $flags
        done
    done
}

# Ctrl-C while one job's failure is allowed and a sibling is still running.
@test "Ctrl-C with an allowed failure in the step is a cancelled run" {
    SLEEPER="30.$$$RANDOM"
    mkdir -p a b
    touch a/module.toml b/module.toml
    echo a > a/f.mod
    echo b > b/f.mod
    cat <<EOF2 > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["ws"] {
        allow_failure = true
        glob = "**/*.mod"
        workspace_indicator = "module.toml"
        dir = "{{workspace}}"
        check = "touch \$PWD/../started; case \$PWD in */a) sleep 1; exit 3;; *) sleep $SLEEPER;; esac"
      }
      ["dependent"] { depends = "ws"; check = "echo DEPENDENT-RAN" }
    }
  }
}
EOF2
    git add hk.pkl a b
    git commit -m "init"

    for libgit2 in 1 0; do
        export HK_LIBGIT2=$libgit2
        _interrupt_and_expect_cancelled started
    done
}
