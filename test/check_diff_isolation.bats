#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

_setup_isolation() {
    printf 'input\n' > a.in
    printf 'input\n' > b.in
    printf 'before\n' > shared.txt
    cat > wait.py <<'PY'
import pathlib
import sys
import time

path = pathlib.Path(sys.argv[1])
expected = sys.argv[2] if len(sys.argv) > 2 else None
started = time.monotonic()
while True:
    observed = path.exists() and (expected is None or path.read_text().strip() == expected)
    if observed or time.monotonic() - started >= 2:
        break
    time.sleep(0.01)
with open('wait-timings.log', 'a') as log:
    log.write(f'{sys.argv[1:]}: observed={observed}, elapsed={time.monotonic() - started:.3f}s\n')
sys.exit(0 if observed else 1)
PY
    cat > a.patch <<'PATCH'
--- shared.txt
+++ shared.txt
@@ -1 +1 @@
-before
+after
PATCH
}

_run_isolated_fix() {
    run python3 - "$@" <<'PY'
import os
import pathlib
import signal
import subprocess
import sys

process = subprocess.Popen(['hk', 'fix', 'a.in', 'b.in', *sys.argv[1:]], start_new_session=True)
try:
    sys.exit(process.wait(timeout=15))
except subprocess.TimeoutExpired:
    os.killpg(process.pid, signal.SIGKILL)
    process.wait()
    print('concurrent fix timed out after 15 seconds', file=sys.stderr)
    sys.exit(1)
finally:
    timings = pathlib.Path('wait-timings.log')
    if timings.exists():
        print(timings.read_text(), end='', file=sys.stderr)
PY
}

_assert_command_excludes_patch() {
    _setup_isolation
    local check_diff_effect="$1"
    cat > hk.pkl <<EOF_CONFIG
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["a"] {
                glob = "a.in"
                check_diff = new CommandSpec { command = "python3 wait.py b-started && cat a.patch; exit 1"; effect = "$check_diff_effect" }
                fix = "touch a-fixer-ran"
            }
            ["b"] {
                glob = "b.in"
                fix = "touch b-started; if python3 wait.py shared.txt after; then touch overlapped; fi; echo B > shared.txt"
            }
        }
    }
}
EOF_CONFIG
    _run_isolated_fix
    assert_success
    assert_file_not_exists overlapped
    assert_file_exists a-fixer-ran
    assert_file_contains shared.txt '^B$'
}

@test "patches outside job inputs wait for ordinary commands" {
    _assert_command_excludes_patch write
}

@test "read-only diffs retain isolation after upgrading file locks" {
    _assert_command_excludes_patch read
}

@test "ordinary fixers on disjoint inputs still run concurrently" {
    _setup_isolation
    cat > hk.pkl <<EOF_CONFIG
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["a"] { glob = "a.in"; fix = "touch a-started; python3 wait.py b-started && echo fixed > a.in" }
            ["b"] { glob = "b.in"; fix = "touch b-started; python3 wait.py a-started && echo fixed > b.in" }
        }
    }
}
EOF_CONFIG
    _run_isolated_fix
    assert_success
    assert_file_contains a.in '^fixed$'
    assert_file_contains b.in '^fixed$'
}

_assert_followup_isolation() {
    _setup_isolation
    local check_after_diff="$1"
    if [ "$check_after_diff" = false ]; then
        # A rejected patch triggers the fallback fixer without modifying files.
        printf 'different\n' > shared.txt
    fi
    cat > b.patch <<'PATCH'
--- b.in
+++ b.in
@@ -1 +1 @@
-input
+B
PATCH
    cat > hk.pkl <<EOF_CONFIG
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["a"] {
                glob = "a.in"
                check_diff = "python3 wait.py c-started && cat a.patch; exit 1"
                check_after_diff = $check_after_diff
                check = "sh followup.sh"
                fix = "sh followup.sh; touch a-fixer-ran"
            }
            ["c"] {
                glob = "b.in"
                fix = "touch c-started; python3 wait.py shared.txt after || true"
            }
            ["b"] {
                glob = "b.in"
                depends = List("c")
                check_diff = "python3 wait.py followup-started && cat b.patch; exit 1"
            }
        }
    }
}
EOF_CONFIG
    cat > followup.sh <<'SCRIPT'
touch followup-started
if python3 wait.py b.in B; then touch overlapped; fi
touch followup-finished
SCRIPT
    _run_isolated_fix
    assert_success
    assert_file_not_exists overlapped
    assert_file_exists followup-finished
    assert_file_contains b.in '^B$'
}

@test "fallback fixer reacquires access before another patch applies" {
    _assert_followup_isolation false
    assert_file_exists a-fixer-ran
}

@test "check_after_diff reacquires access before another patch applies" {
    _assert_followup_isolation true
    assert_file_not_exists a-fixer-ran
    assert_file_contains shared.txt '^after$'
}

@test "staging waits for an in-process patch outside the job inputs" {
    case "$OSTYPE" in
        msys*|cygwin*|win*) skip "requires a FIFO and a Unix executable shim" ;;
    esac
    _setup_isolation
    # Staging must not incidentally wait on A's input-file lock.
    python3 - <<'PY'
import os
for name in ('a.in', 'b.in', 'shared.txt'):
    os.utime(name, (1, 1))
PY
    git add a.in b.in shared.txt
    printf 'unstaged\n' > shared.txt
    cat > a.patch <<'PATCH'
--- shared.txt
+++ shared.txt
@@ -1 +1 @@
-unstaged
+after
--- gate
+++ /dev/null
@@ -1 +0,0 @@
-input
PATCH
    mkfifo gate
    export REAL_GIT
    REAL_GIT=$(command -v git)
    mkdir mock-bin
    # Rendezvous before staging takes its shared guard. The FIFO holds the
    # real in-process patch inside snapshot preparation until staging tries
    # to run; no git-apply shim or injected patch implementation is involved.
    cat > mock-bin/git <<'SCRIPT'
#!/bin/sh
if [ "$1" = rev-parse ] && [ "$2" = --git-path ] && [ "$3" = index ]; then
    touch staging-started
    python3 wait.py patch-reading || exit 1
fi
exec "$REAL_GIT" "$@"
SCRIPT
    chmod +x mock-bin/git
    export PATH="$PWD/mock-bin:$PATH"
    cat > hold-patch.py <<'PY'
import os
import pathlib
import subprocess
import time

with open('gate', 'w') as gate:
    gate.write('input\n')
    gate.flush()
    pathlib.Path('patch-reading').touch()
    # Without isolation, staging can return while the patch is incomplete.
    # Wait by elapsed time, keeping the FIFO open to delay EOF.
    deadline = time.monotonic() + 2
    while time.monotonic() < deadline:
        indexed = subprocess.check_output([os.environ['REAL_GIT'], 'show', ':shared.txt'])
        if indexed == b'unstaged\n':
            pathlib.Path('staged-early').touch()
            break
        time.sleep(0.01)
PY
    cat > hk.pkl <<EOF_CONFIG
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["a"] {
                glob = "a.in"
                stage = List()
                fix = "exit 99"
                check_diff = "python3 wait.py staging-started && cat a.patch; exit 1"
            }
            ["b"] {
                glob = "b.in"
                stage = "shared.txt"
                fix = "echo fixed > b.in"
            }
        }
    }
}
EOF_CONFIG
    # Launch the FIFO helper inside the same process group as hk so the outer
    # timeout also cleans it up if lock ordering regresses.
    cat > launch.py <<'PY'
import subprocess
import sys

helper = subprocess.Popen(['python3', 'hold-patch.py'])
try:
    sys.exit(subprocess.call(['hk', 'fix', 'a.in', 'b.in', '--stage']))
finally:
    helper.terminate()
    helper.wait()
PY
    run python3 - <<'PY'
import os
import signal
import subprocess
import sys
process = subprocess.Popen(['python3', 'launch.py'], start_new_session=True)
try:
    sys.exit(process.wait(timeout=15))
except subprocess.TimeoutExpired:
    os.killpg(process.pid, signal.SIGKILL)
    process.wait()
    sys.exit(1)
PY
    assert_success
    assert_file_exists patch-reading
    assert_file_not_exists staged-early
    assert_file_contains shared.txt '^after$'
    run git show :shared.txt
    assert_output after
}
