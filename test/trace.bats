#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    cat >hk.toml <<EOF
[[hook.check.steps]]
inline = 'echo "test check"'
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

@test "trace: cache hit/miss events" {
    # First run should be cache miss
    rm -rf .hk/cache
    run hk --trace --json check
    assert_success
    assert_output --partial '"name":"cache.miss"'

    # Second run should be cache hit
    run hk --trace --json check
    assert_success
    assert_output --partial '"name":"cache.hit"'
}

@test "trace: git operations" {
    echo "test" > test.txt
    run hk --trace --json check
    assert_success
    assert_output --partial '"name":"git.status"'
}

@test "trace: with stashing" {
    echo "unstaged" > test.txt
    run hk --trace --json check --stash=git
    assert_success
    assert_output --partial '"name":"git.stash.push"'
}
