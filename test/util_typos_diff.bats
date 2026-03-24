#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    PATH="$PATH":"$PROJECT_ROOT"/test/builtin_tool_stubs
}

teardown() {
    _common_teardown
}

@test "util typos-diff - reports diff and exits non-zero" {
    printf "maintainance\n" > test.txt

    run hk util typos-diff test.txt
    assert_failure
    assert_output --partial "-maintainance"
    assert_output --partial "+maintenance"
}

@test "util typos-diff - passes clean files" {
    printf "maintenance\n" > test.txt

    run hk util typos-diff test.txt
    assert_success
    refute_output
}
