#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util trailing-whitespace - detects trailing whitespace" {
    echo "clean line" > file1.txt
    echo "trailing  " >> file1.txt

    run hk util trailing-whitespace file1.txt
    assert_failure
    assert_output "file1.txt"
}

@test "util trailing-whitespace - passes clean files" {
    echo "clean line" > file1.txt
    echo "another clean line" >> file1.txt

    run hk util trailing-whitespace file1.txt
    assert_success
    refute_output
}

@test "util trailing-whitespace - fixes trailing whitespace" {
    echo "clean line" > file1.txt
    echo "trailing  " >> file1.txt
    echo "more trailing	" >> file1.txt

    run hk util trailing-whitespace --fix file1.txt
    assert_failure # exits 1 when changes are made

    # Verify file was fixed
    run cat file1.txt
    assert_output "clean line
trailing
more trailing"
}

@test "util trailing-whitespace - multiple files" {
    echo "trailing  " > file1.txt
    echo "also trailing  " > file2.txt

    run hk util trailing-whitespace file1.txt file2.txt
    assert_failure
    assert_output --partial "file1.txt"
    assert_output --partial "file2.txt"
}

@test "util trailing-whitespace - fix multiple files" {
    echo "trailing  " > file1.txt
    echo "also trailing  " > file2.txt

    run hk util trailing-whitespace --fix file1.txt file2.txt
    assert_failure # exits 1 when changes are made

    # Verify both files were fixed
    run cat file1.txt
    assert_output "trailing"

    run cat file2.txt
    assert_output "also trailing"
}

@test "util trailing-whitespace - skips non-text files" {
    # Create a binary file
    printf '\x00\x01\x02\x03' > binary.bin

    # Should not fail on binary files
    run hk util trailing-whitespace binary.bin
    assert_success
}

@test "util trailing-whitespace - detects various whitespace types" {
    echo "space trailing " > file1.txt
    echo "tab trailing	" >> file1.txt
    echo "mixed   	" >> file1.txt

    run hk util trailing-whitespace file1.txt
    assert_failure
    assert_output "file1.txt"
}

@test "util trailing-whitespace - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["trailing-ws"] = Builtins.trailing_whitespace
        }
    }
}
HK

    echo "trailing  " > test.txt

    run hk check
    assert_failure

    # Fix should clean it
    run hk fix
    assert_success

    run cat test.txt
    assert_output "trailing"
}
