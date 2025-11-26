#!/usr/bin/env mise run test:bats
setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "builtins lists all available builtin linters" {
    run hk builtins

    assert_success
    assert_line --regexp '^actionlint$'
    assert_line --regexp '^prettier$'
    assert_line --regexp '^rustfmt$'
    assert_line --regexp '^yamllint$'
}

@test "builtins output is sorted" {
    run hk builtins

    assert_success
    # Check that the output is sorted alphabetically
    assert_output --partial 'actionlint'
    assert_output --partial 'prettier'
    assert_output --partial 'rustfmt'
    assert_output --partial 'yamllint'
}

@test "builtins works in any directory" {
    cd "$TEST_TEMP_DIR"
    run hk builtins

    assert_success
    assert_line --regexp '^actionlint$'
    assert_line --regexp '^prettier$'
}
