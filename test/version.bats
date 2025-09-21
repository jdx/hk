#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hk --version prints version" {
    assert_hk_success --version
    assert_output --regexp "^hk\ [0-9]+\.[0-9]+\.[0-9]+$"
} 
