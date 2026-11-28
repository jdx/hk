#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "HK_OUTPUT_FILE writes failed command output to a custom path" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] {
        check = "echo CUSTOM_OUTPUT_FILE_MARKER 1>&2 && exit 1"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    output_file="$BATS_TEST_TMPDIR/nested/logs/failed-output.log"
    HK_OUTPUT_FILE="$output_file" run hk check

    assert_failure
    assert_output --partial "See $output_file for full command output"
    assert_file_exist "$output_file"
    assert_file_contains "$output_file" "CUSTOM_OUTPUT_FILE_MARKER"
    assert_file_not_exist "$HK_STATE_DIR/output.log"
}

@test "empty HK_OUTPUT_FILE uses the default location" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] {
        check = "echo EMPTY_OUTPUT_FILE_MARKER 1>&2 && exit 1"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt
    export HK_STATE_DIR="$BATS_TEST_TMPDIR/hk_state"

    HK_OUTPUT_FILE="" run hk check

    output_file="$HK_STATE_DIR/output.log"
    assert_failure
    assert_output --partial "See $output_file for full command output"
    assert_file_exist "$output_file"
    assert_file_contains "$output_file" "EMPTY_OUTPUT_FILE_MARKER"
}

@test "HK_OUTPUT_FILE writes to a bare filename" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] {
        check = "echo BARE_OUTPUT_FILE_MARKER 1>&2 && exit 1"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt

    HK_OUTPUT_FILE="failed-output.log" run hk check

    assert_failure
    assert_output --partial "See failed-output.log for full command output"
    assert_file_exist "failed-output.log"
    assert_file_contains "failed-output.log" "BARE_OUTPUT_FILE_MARKER"
}

@test "output.log defaults to the checkout's git directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] {
        check = "echo GIT_DIR_MARKER 1>&2 && exit 1"
        glob = List("*.txt")
      }
    }
  }
}
EOF
    echo "content" > file.txt
    git add file.txt hk.pkl
    git commit -m init -q
    git worktree add -q "$BATS_TEST_TMPDIR/wt" -b wt
    unset HK_STATE_DIR HK_OUTPUT_FILE HK_LOG_FILE

    run hk check --all
    assert_failure
    assert_output --regexp "See .*/\.git/hk/output\.log for full command output"
    assert_file_contains ".git/hk/output.log" "GIT_DIR_MARKER"

    # A linked worktree keeps its own output.log
    cd "$BATS_TEST_TMPDIR/wt"
    run hk check --all
    assert_failure
    wt_git_dir="$(git rev-parse --absolute-git-dir)"
    assert_output --regexp "See .*/\.git/worktrees/wt/hk/output\.log for full command output"
    assert_file_contains "$wt_git_dir/hk/output.log" "GIT_DIR_MARKER"
}

@test "output file holds every step's summary when several steps fail" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail-a"] {
        check = #"printf '\033[31mFIRST_FAILURE_MARKER\033[0m\n' && exit 1"#
      }
      ["fail-b"] {
        check = "echo SECOND_FAILURE_MARKER 1>&2 && exit 1"
      }
      ["passes"] {
        check = "echo PASSING_STEP_MARKER 1>&2"
      }
      ["hidden"] {
        check = "echo HIDDEN_STEP_MARKER && exit 1"
        output_summary = "hide"
      }
    }
  }
}
EOF
    echo "content" > file.txt
    output_file="$BATS_TEST_TMPDIR/output.log"

    HK_OUTPUT_FILE="$output_file" run hk check --all --no-fail-fast

    assert_failure
    assert_file_contains "$output_file" "fail-a stderr (failed):"
    assert_file_contains "$output_file" "FIRST_FAILURE_MARKER"
    assert_file_contains "$output_file" "fail-b stderr (failed):"
    assert_file_contains "$output_file" "SECOND_FAILURE_MARKER"
    assert_file_contains "$output_file" "passes stderr (passed):"
    assert_file_contains "$output_file" "PASSING_STEP_MARKER"
    run grep -c $'\033' "$output_file"
    assert_output "0"
    run grep -c HIDDEN_STEP_MARKER "$output_file"
    assert_output "0"
}

@test "output file only describes the latest run" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["step"] {
        check = "if test -f fail; then echo STALE_FAILURE_MARKER 1>&2; exit 1; else echo FRESH_PASS_MARKER 1>&2; fi"
      }
    }
  }
}
EOF
    output_file="$BATS_TEST_TMPDIR/output.log"
    touch fail

    HK_OUTPUT_FILE="$output_file" run hk check --all
    assert_failure
    assert_file_contains "$output_file" "STALE_FAILURE_MARKER"

    rm fail
    HK_OUTPUT_FILE="$output_file" run hk check --all
    assert_success
    assert_file_contains "$output_file" "step stderr (passed):"
    assert_file_contains "$output_file" "FRESH_PASS_MARKER"
    run grep -c STALE_FAILURE_MARKER "$output_file"
    assert_output "0"

    # A run where no step prints anything leaves no file behind.
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["quiet"] { check = "true" }
    }
  }
}
EOF
    HK_OUTPUT_FILE="$output_file" run hk check --all
    assert_success
    assert_file_not_exist "$output_file"
}

@test "a hidden step's failure stays out of the output file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["hidden"] {
        check = "echo ONLY_HIDDEN_FAILURE_MARKER 1>&2 && exit 1"
        output_summary = "hide"
      }
    }
  }
}
EOF
    echo "content" > file.txt
    output_file="$BATS_TEST_TMPDIR/output.log"

    HK_OUTPUT_FILE="$output_file" run hk check --all

    assert_failure
    assert_file_not_exist "$output_file"
    refute_output --partial "See $output_file"
}

@test "a step's rendered diagnostics are not followed by its raw output" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["eslint"] {
        check = "cat report.json; exit 1"
        output_summary = "combined"
        diagnostic_format = "eslint-json"
      }
    }
  }
}
EOF
    cat <<'JSON' > report.json
[{"filePath":"input.js","messages":[{"ruleId":"demo","severity":2,"message":"RENDERED_DIAGNOSTIC_MARKER","line":1,"column":1}]}]
JSON
    output_file="$BATS_TEST_TMPDIR/output.log"

    HK_OUTPUT_FILE="$output_file" run hk check --all

    assert_failure
    assert_file_contains "$output_file" "RENDERED_DIAGNOSTIC_MARKER"
    run grep -c -e "failed command output" -e '"filePath"' "$output_file"
    assert_output "0"
}

@test "a step that fails silently leaves no output file" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["silent"] {
        check = "exit 1"
        output_summary = "hide"
      }
    }
  }
}
EOF
    echo "content" > file.txt
    output_file="$BATS_TEST_TMPDIR/output.log"

    HK_OUTPUT_FILE="$output_file" run hk check --all

    assert_failure
    assert_file_not_exist "$output_file"
    refute_output --partial "See $output_file"
}

@test "output directory holds one file per step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail-a"] {
        check = #"printf '\033[31mSTEP_A_MARKER\033[0m\n' 1>&2 && exit 1"#
      }
      ["lint/js"] {
        check = "echo STEP_B_MARKER 1>&2 && exit 1"
      }
      ["passes"] {
        check = "echo PASSING_STEP_MARKER 1>&2"
      }
      ["quiet"] {
        check = "true"
      }
      ["hidden"] {
        check = "echo HIDDEN_STEP_MARKER 1>&2 && exit 1"
        output_summary = "hide"
      }
    }
  }
}
EOF
    output_dir="$BATS_TEST_TMPDIR/out"

    HK_OUTPUT_DIR="$output_dir" run hk check --all --no-fail-fast

    assert_failure
    assert_output --partial "or $output_dir for one file per step"
    run ls "$output_dir"
    assert_output "$(printf 'fail-a.log\nlint%%2Fjs.log\npasses.log')"
    run cat "$output_dir/fail-a.log"
    assert_output "$(printf 'fail-a stderr (failed):\nSTEP_A_MARKER')"
    assert_file_contains "$output_dir/lint%2Fjs.log" "lint/js stderr (failed):"
    assert_file_contains "$output_dir/lint%2Fjs.log" "STEP_B_MARKER"
    assert_file_contains "$output_dir/passes.log" "passes stderr (passed):"
    run grep -l STEP_B_MARKER "$output_dir/fail-a.log" "$output_dir/passes.log"
    assert_output ""
}

@test "output directory only describes the latest run" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["old-step"] { check = "echo OLD_STEP_MARKER 1>&2" }
    }
  }
}
EOF
    output_dir="$BATS_TEST_TMPDIR/out"

    HK_OUTPUT_DIR="$output_dir" run hk check --all
    assert_success
    assert_file_exist "$output_dir/old-step.log"
    echo keep > "$output_dir/notes.txt"

    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["new-step"] { check = "echo NEW_STEP_MARKER 1>&2" }
    }
  }
}
EOF
    HK_OUTPUT_DIR="$output_dir" run hk check --all
    assert_success
    assert_file_not_exist "$output_dir/old-step.log"
    assert_file_contains "$output_dir/new-step.log" "NEW_STEP_MARKER"
    assert_file_exist "$output_dir/notes.txt"

    # A run with no step output removes the step files, and the directory once
    # nothing else is in it.
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["quiet"] { check = "true" }
    }
  }
}
EOF
    HK_OUTPUT_DIR="$output_dir" run hk check --all
    assert_success
    assert_file_not_exist "$output_dir/new-step.log"
    assert_file_exist "$output_dir/notes.txt"
    rm "$output_dir/notes.txt"
    HK_OUTPUT_DIR="$output_dir" run hk check --all
    assert_success
    assert_dir_not_exist "$output_dir"
}

@test "output directory defaults to the checkout's git directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] { check = "echo GIT_DIR_STEP_MARKER 1>&2 && exit 1" }
    }
  }
}
EOF
    unset HK_STATE_DIR HK_OUTPUT_FILE HK_OUTPUT_DIR HK_LOG_FILE

    run hk check --all
    assert_failure
    assert_output --regexp "or .*/\.git/hk/output for one file per step"
    assert_file_contains ".git/hk/output/fail.log" "GIT_DIR_STEP_MARKER"
    assert_file_exists ".git/hk/output.lock"
}

@test "runs with different state directories share one output directory's lock" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] { check = "echo SHARED_MARKER 1>&2 && exit 1" }
    }
  }
}
EOF
    export HK_OUTPUT_DIR="$TEST_TEMP_DIR/shared-output"

    HK_STATE_DIR="$TEST_TEMP_DIR/state-a" run hk check --all
    assert_failure
    HK_STATE_DIR="$TEST_TEMP_DIR/state-b" run hk check --all
    assert_failure

    assert_file_contains "$HK_OUTPUT_DIR/fail.log" "SHARED_MARKER"
    assert_file_exists "$TEST_TEMP_DIR/shared-output.lock"
    run find "$TEST_TEMP_DIR/state-a" "$TEST_TEMP_DIR/state-b" -name 'output-*.lock'
    assert_output ""
}

@test "a run waits for the output lock held by a run with another state directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fail"] { check = "echo WAITED_MARKER 1>&2 && exit 1" }
    }
  }
}
EOF
    mkdir -p "$TEST_TEMP_DIR/shared/child"
    # `..` is resolved before the lock is named, so this locks shared.lock.
    export HK_OUTPUT_DIR="$TEST_TEMP_DIR/shared/child/.."
    # Hold the lock as another run would, until "release" exists.
    python3 - "$TEST_TEMP_DIR/shared.lock" "$TEST_TEMP_DIR" <<'PY' &
import fcntl, os, sys, time
lock = open(sys.argv[1], "a")
fcntl.flock(lock, fcntl.LOCK_EX)
open(os.path.join(sys.argv[2], "held"), "w").close()
while not os.path.exists(os.path.join(sys.argv[2], "release")):
    time.sleep(0.05)
PY
    holder=$!
    for _ in $(seq 100); do [ -e "$TEST_TEMP_DIR/held" ] && break; sleep 0.1; done
    held=no; [ -e "$TEST_TEMP_DIR/held" ] && held=yes

    HK_STATE_DIR="$TEST_TEMP_DIR/state-b" hk check --all >/dev/null 2>&1 &
    waiter=$!
    sleep 1
    # Recorded before releasing, so a failed assertion never leaves the holder running.
    written_while_held=no; [ -e "$TEST_TEMP_DIR/shared/fail.log" ] && written_while_held=yes

    touch "$TEST_TEMP_DIR/release"
    wait "$holder"
    wait "$waiter" || true
    assert_equal "$held" yes
    assert_equal "$written_while_held" no
    assert_file_contains "$TEST_TEMP_DIR/shared/fail.log" "WAITED_MARKER"
}

@test "a step file that would be the output file is not written" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] { check = "echo LINT_MARKER 1>&2" }
      ["format"] { check = "echo FORMAT_MARKER 1>&2" }
    }
  }
}
EOF
    output_dir="$BATS_TEST_TMPDIR/out"

    HK_OUTPUT_DIR="$output_dir" HK_OUTPUT_FILE="$output_dir/lint.log" run hk check --all

    assert_success
    assert_output --partial "is HK_OUTPUT_FILE"
    assert_file_contains "$output_dir/lint.log" "format stderr (passed):"
    assert_file_contains "$output_dir/lint.log" "LINT_MARKER"
    run cat "$output_dir/format.log"
    assert_output "$(printf 'format stderr (passed):\nFORMAT_MARKER')"
}
