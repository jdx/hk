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

    # CRLF is the most frequent ending, so the file is normalized to CRLF
    assert_equal "$(od -An -c file.txt | tr -s ' ')" "$(printf 'line1\r\nline2\r\nline3\r\n' | od -An -c | tr -s ' ')"
}

@test "util mixed-line-ending - fixes to LF when LF is the most frequent" {
    printf "line1\nline2\r\nline3\n" > file.txt

    run hk util mixed-line-ending --fix file.txt
    assert_success
    refute_output

    assert_equal "$(od -An -c file.txt | tr -s ' ')" "$(printf 'line1\nline2\nline3\n' | od -An -c | tr -s ' ')"
}

@test "util mixed-line-ending - a tie normalizes to LF" {
    printf "line1\r\nline2\n" > file.txt

    run hk util mixed-line-ending --fix file.txt
    assert_success

    assert_equal "$(od -An -c file.txt | tr -s ' ')" "$(printf 'line1\nline2\n' | od -An -c | tr -s ' ')"
}

@test "util mixed-line-ending - fix leaves a file with one ending alone" {
    printf "line1\r\nline2\r\n" > crlf.txt

    run hk util mixed-line-ending --fix crlf.txt
    assert_success

    assert_equal "$(od -An -c crlf.txt | tr -s ' ')" "$(printf 'line1\r\nline2\r\n' | od -An -c | tr -s ' ')"
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

    # Each file is tied between the endings, so both are normalized to LF
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

@test "util mixed-line-ending - diff mode outputs unified diff" {
    printf "line1\r\nline2\nline3\r\n" > file.txt

    run hk util mixed-line-ending --diff file.txt
    assert_failure
    # CRLF is the most frequent ending, so the diff rewrites the one LF
    printf -- "--- a/file.txt\n+++ b/file.txt\n@@ -1,3 +1,3 @@\n line1\r\n-line2\n+line2\r\n line3\r\n" | assert_output
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

    # Verify file was normalized to its most frequent ending
    assert_equal "$(od -An -c test.txt | tr -s ' ')" "$(printf 'line1\r\nline2\r\nline3\r\n' | od -An -c | tr -s ' ')"
}
