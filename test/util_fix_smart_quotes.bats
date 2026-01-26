#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util fix-smart-quotes --check - detects smart quotes" {
    python3 -c "print('This has \u201Csmart quotes\u201D')" > file1.txt

    run hk util fix-smart-quotes --check file1.txt
    assert_failure
    assert_output --partial "file1.txt"
}

@test "util fix-smart-quotes --check - passes clean files" {
    echo 'This has "normal quotes"' > file1.txt

    run hk util fix-smart-quotes --check file1.txt
    assert_success
    refute_output
}

@test "util fix-smart-quotes - fixes smart quotes" {
    python3 -c "print('This has \u201Csmart quotes\u201D', end='')" > file1.txt

    run hk util fix-smart-quotes file1.txt
    assert_success

    # Verify file was fixed
    run cat file1.txt
    assert_output 'This has "smart quotes"'
}

@test "util fix-smart-quotes --check - multiple files" {
    python3 -c "print('\u201Cquote\u201D', end='')" > file1.txt
    python3 -c "print('\u2018apostrophe\u2019', end='')" > file2.txt

    run hk util fix-smart-quotes --check file1.txt file2.txt
    assert_failure
    assert_output --partial "file1.txt"
    assert_output --partial "file2.txt"
}

@test "util fix-smart-quotes - fix multiple files" {
    python3 -c "print('\u201Cquote\u201D', end='')" > file1.txt
    python3 -c "print('\u2018apostrophe\u2019', end='')" > file2.txt

    run hk util fix-smart-quotes file1.txt file2.txt
    assert_success

    # Verify both files were fixed
    run cat file1.txt
    assert_output '"quote"'

    run cat file2.txt
    assert_output "'apostrophe'"
}

@test "util fix-smart-quotes --check - empty file passes" {
    touch file1.txt

    run hk util fix-smart-quotes --check file1.txt
    assert_success
}

@test "util fix-smart-quotes --check - detects various smart quote types" {
    # Test fullwidth double quotes
    python3 -c "print('\uFF02fullwidth\uFF02', end='')" > file1.txt
    run hk util fix-smart-quotes --check file1.txt
    assert_failure

    # Test fullwidth single quotes
    python3 -c "print('\uFF07fullwidth apostrophe\uFF07', end='')" > file2.txt
    run hk util fix-smart-quotes --check file2.txt
    assert_failure
}

@test "util fix-smart-quotes --diff - outputs unified diff" {
    python3 -c "print('\u201Csmart\u201D')" > file1.txt

    run hk util fix-smart-quotes --diff file1.txt
    assert_failure
    assert_output --partial '--- a/file1.txt
+++ b/file1.txt
@@ -1 +1 @@
-“smart”
+"smart"'
}
