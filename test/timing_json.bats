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
    step_ms=$(jq -r '.steps["a"]' "$timing_file")

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
