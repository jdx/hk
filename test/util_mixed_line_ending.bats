#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util mixed-line-ending - detects mixed endings" {
    printf "line1\r\nline2\nline3\r\n" > file.txt

    run hk util mixed-line-ending file.txt
    assert_failure
    assert_output --partial "file.txt"
}

@test "util mixed-line-ending - passes LF only" {
    printf "line1\nline2\nline3\n" > file.txt

    run hk util mixed-line-ending file.txt
    assert_success
    refute_output
}

@test "util mixed-line-ending - passes CRLF only" {
    printf "line1\r\nline2\r\nline3\r\n" > file.txt

    run hk util mixed-line-ending file.txt
    assert_success
    refute_output
}

@test "util mixed-line-ending - fixes mixed endings" {
    printf "line1\r\nline2\nline3\r\n" > file.txt

    run hk util mixed-line-ending --fix file.txt
    assert_success
    refute_output

    # Verify file was normalized to LF
    run cat file.txt
    assert_output "$(printf "line1\nline2\nline3\n")"
}

@test "util mixed-line-ending - multiple files" {
    printf "line1\r\nline2\n" > file1.txt
    printf "line1\nline2\r\n" > file2.txt

    run hk util mixed-line-ending file1.txt file2.txt
    assert_failure
    assert_output --partial "file1.txt"
    assert_output --partial "file2.txt"
}

@test "util mixed-line-ending - fix multiple files" {
    printf "line1\r\nline2\n" > file1.txt
    printf "line1\nline2\r\n" > file2.txt

    run hk util mixed-line-ending --fix file1.txt file2.txt
    assert_success
    refute_output

    # Verify both files normalized
    run cat file1.txt
    assert_output "$(printf "line1\nline2\n")"
    run cat file2.txt
    assert_output "$(printf "line1\nline2\n")"
}

@test "util mixed-line-ending - skips binary files" {
    printf "binary\x00data\r\nwith\nlines" > binary.bin

    run hk util mixed-line-ending binary.bin
    assert_success
    refute_output
}

@test "util mixed-line-ending - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["mixed-endings"] = Builtins.mixed_line_ending
        }
    }
}
HK

    printf "line1\r\nline2\nline3\r\n" > test.txt

    run hk check
    assert_failure
    assert_output --partial "test.txt"
}

@test "util mixed-line-ending - builtin fix integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["fix"] {
        steps {
            ["mixed-endings"] = Builtins.mixed_line_ending
        }
    }
}
HK

    printf "line1\r\nline2\nline3\r\n" > test.txt

    run hk fix
    assert_success

    # Verify file was normalized
    run cat test.txt
    assert_output "$(printf "line1\nline2\nline3\n")"
}
