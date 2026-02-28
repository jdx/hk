#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util end-of-file-fixer - detects missing final newline" {
    printf "no newline" > file.txt

    run hk util end-of-file-fixer file.txt
    assert_failure
    assert_output --partial "file.txt"
}

@test "util end-of-file-fixer - passes file with final newline" {
    printf "has newline\n" > file.txt

    run hk util end-of-file-fixer file.txt
    assert_success
    refute_output
}

@test "util end-of-file-fixer - fixes missing final newline" {
    printf "no newline" > file.txt

    run hk util end-of-file-fixer --fix file.txt
    assert_success

    run cat file.txt
    assert_output "no newline"
    # Verify file ends with newline (cat strips it from assert_output)
    run sh -c 'tail -c 1 file.txt | od -c'
    assert_output --partial '\n'
}

@test "util end-of-file-fixer - diff mode outputs unified diff" {
    printf "no newline" > file.txt

    run hk util end-of-file-fixer --diff file.txt
    assert_failure
    assert_output  "--- a/file.txt
+++ b/file.txt
@@ -1 +1 @@
-no newline
\ No newline at end of file
+no newline"
}

@test "util end-of-file-fixer - detects extra trailing newlines" {
    printf "content\n\n\n" > file.txt

    run hk util end-of-file-fixer file.txt
    assert_failure
    assert_output --partial "file.txt"
}

@test "util end-of-file-fixer - fixes extra trailing newlines" {
    printf "content\n\n\n" > file.txt

    run hk util end-of-file-fixer --fix file.txt
    assert_success

    # Verify file ends with exactly one newline
    run cat file.txt
    assert_output "content"
    run sh -c 'wc -c < file.txt'
    assert_output --partial "8"
}

@test "util end-of-file-fixer - skips binary files" {
    printf "binary\x00data" > binary.bin

    run hk util end-of-file-fixer binary.bin
    assert_success
    refute_output
}

@test "util end-of-file-fixer - empty file passes" {
    touch empty.txt

    run hk util end-of-file-fixer empty.txt
    assert_success
    refute_output
}
