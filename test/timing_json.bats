#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "HK_TIMING_JSON reports per-step wall time (batch overlap)" {
    # require jq for JSON parsing
    type -p jq &>/dev/null || skip "jq is required"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] {
        glob = List("*.txt")
        batch = true
        check = "sleep 1; echo checking {{files}}"
    } } }
}
EOF
    # create multiple files to trigger batching into exactly 2 jobs (with HK_JOBS=2)
    echo "1" > f1.txt
    echo "2" > f2.txt
    echo "3" > f3.txt
    echo "4" > f4.txt

    timing_file="$TEST_TEMP_DIR/timing.json"
    export HK_TIMING_JSON="$timing_file"

    run hk check --all
    assert_success
    assert_file_exists "$timing_file"

    # extract total wall time
    total_ms=$(jq -r '.total.wall_time_ms' "$timing_file")
    # extract step wall time for step name "a" (steps is an object keyed by step name)
    step_ms=$(jq -r '.steps["a"].wall_time_ms' "$timing_file")

    # sanity checks
    [ -n "$total_ms" ]
    [ -n "$step_ms" ]

    # step wall time should be roughly ~1s (since two 1s batches overlap)
    # allow generous bounds for CI jitter
    [ "$step_ms" -ge 700 ]
    [ "$step_ms" -lt 1800 ]

    # total wall time should be at least the per-step time
    [ "$total_ms" -ge "$step_ms" ]
}

@test "interactive field in json timing report" {
    # require jq for JSON parsing
    type -p jq &>/dev/null || skip "jq is required"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        fix = true
        stash = "git"
        steps {
            ["interactive-step"] {
                interactive = true
                fix = "echo 'interactive step'"
            }
            ["normal-step"] {
                fix = "echo 'normal step'"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "init"

    # Create a test file to trigger the hook
    echo "test" > test.txt
    git add test.txt

    timing_file="$TEST_TEMP_DIR/timing.json"
    export HK_TIMING_JSON="$timing_file"

    # Run with timing JSON output
    run hk run pre-commit
    assert_success
    assert_file_exists "$timing_file"

    # Check that the JSON contains the interactive field for both steps
    assert_file_contains "$timing_file" '"interactive": true'
    assert_file_contains "$timing_file" '"interactive": false'

    # Verify with jq that the interactive field is correctly set
    interactive_step=$(jq -r '.steps["interactive-step"].interactive' "$timing_file")
    normal_step=$(jq -r '.steps["normal-step"].interactive' "$timing_file")

    [ "$interactive_step" = "true" ]
    [ "$normal_step" = "false" ]
}

@test "hook-level report receives HK_REPORT_JSON" {
    type -p jq &>/dev/null || skip "jq is required"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps { ["a"] { check = "echo ok" } }
        report = "printf '%s' \"$HK_REPORT_JSON\" | jq -r '.total.wall_time_ms | tostring' >/dev/null"
    }
}
EOF
    run hk check --all
    assert_success
}
