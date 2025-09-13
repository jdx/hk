#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["test"] { check = "echo 'test check'" } } }
}
EOF
    git add .
    git commit -m "initial"
}

@test "trace: pretty output to stderr" {
    run hk --trace check
    assert_success
    assert_output --partial "config.load"
    assert_output --partial "hook.run"
}

@test "trace: JSON output to stdout" {
    run hk --trace --json check
    assert_success

    # Check for metadata line
    assert_line --index 0 --partial '"type":"meta"'
    assert_line --index 0 --partial '"span_schema_version":1'
    assert_line --index 0 --partial '"hk_version":'

    # Check for key spans
    assert_output --partial '"type":"span_start"'
    assert_output --partial '"name":"config.load"'
    assert_output --partial '"name":"hook.run"'
    assert_output --partial '"type":"span_end"'
}

@test "trace: disabled by default" {
    run hk check
    assert_success
    refute_output --partial "config.load"
    refute_output --partial '"type":"span_start"'
}

@test "trace: HK_TRACE environment variable" {
    HK_TRACE=1 run hk check
    assert_success
    assert_output --partial "config.load"
    assert_output --partial "hook.run"
}

@test "trace: HK_TRACE=json environment variable" {
    HK_TRACE=json run hk check
    assert_success
    assert_line --index 0 --partial '"type":"meta"'
    assert_output --partial '"type":"span_start"'
}

@test "trace: cache events" {
    # Clear cache and run - should show cache events
    rm -rf .hk/cache
    run hk --trace --json check
    assert_success
    # Should at least show cache operations
    assert_output --partial '"name":"cache.get_or_try_init"'
}

@test "trace: git operations" {
    echo "test" > test.txt
    run hk --trace --json check
    assert_success
    assert_output --partial '"name":"git.status"'
}

@test "trace: git operations with unstaged files" {
    echo "unstaged" > test.txt
    run hk --trace --json check
    assert_success
    assert_output --partial '"name":"git.status"'
}

@test "trace: enabled when HK_LOG=trace" {
    HK_LOG=trace run hk check
    assert_success
    # pretty tracing output should include span names
    assert_output --partial "config.load"
    assert_output --partial "hook.run"
}

@test "trace: enabled when -vv (trace level)" {
    run hk -vv check
    assert_success
    assert_output --partial "config.load"
    assert_output --partial "hook.run"
}

