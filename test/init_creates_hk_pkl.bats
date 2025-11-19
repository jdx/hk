#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk init creates hk.pkl" {
    hk init
    assert_file_contains hk.pkl "linters ="
}
