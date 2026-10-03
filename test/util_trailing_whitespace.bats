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
    assert_output --partial "file1.txt"
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
    assert_success

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
    assert_success

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
    assert_output --partial "file1.txt"
}

@test "util trailing-whitespace - detects trailing whitespace when a multibyte char spans the 8 KiB probe" {
    # 'é' (2 bytes) straddles byte 8192, so the probe alone ends mid-character.
    head -c 8191 /dev/zero | tr '\0' a > multibyte.txt
    printf 'é\ntrailing  \n' >> multibyte.txt

    run hk util trailing-whitespace multibyte.txt
    assert_failure
    assert_output --partial "multibyte.txt"
}

@test "util trailing-whitespace - diff mode outputs unified diff" {
    echo "trailing  " > file1.txt

    run hk util trailing-whitespace --diff file1.txt
    assert_failure
    assert_output "--- a/file1.txt
+++ b/file1.txt
@@ -1 +1 @@
-trailing  \

+trailing"
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
    ["fix"] {
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

@test "util trailing-whitespace - clean CRLF file passes" {
    printf "contents\r\nmore\r\n" > crlf.txt

    run hk util trailing-whitespace crlf.txt
    assert_success
    refute_output

    run hk util trailing-whitespace --diff crlf.txt
    assert_success
    refute_output

    run hk util trailing-whitespace --fix crlf.txt
    assert_success
    assert_equal "$(od -An -c crlf.txt | tr -s ' ')" "$(printf 'contents\r\nmore\r\n' | od -An -c | tr -s ' ')"
}

@test "util trailing-whitespace - works on windows" {
    printf "contents  \r\nmore\r\n" > crlf.txt

    run hk util trailing-whitespace crlf.txt
    assert_failure
    assert_output --partial "crlf.txt"

    # The diff only removes the spaces; the CRLF terminator stays
    run hk util trailing-whitespace --diff crlf.txt
    assert_failure
    printf -- "--- a/crlf.txt\n+++ b/crlf.txt\n@@ -1,2 +1,2 @@\n-contents  \r\n+contents\r\n more\r\n" | assert_output

    run hk util trailing-whitespace --fix crlf.txt
    assert_success
    assert_equal "$(od -An -c crlf.txt | tr -s ' ')" "$(printf 'contents\r\nmore\r\n' | od -An -c | tr -s ' ')"
}

@test "util trailing-whitespace - keeps each line's terminator" {
    printf "a  \r\nb \nc\t\r\nd  " > mixed.txt

    run hk util trailing-whitespace --fix mixed.txt
    assert_success
    assert_equal "$(od -An -c mixed.txt | tr -s ' ')" "$(printf 'a\r\nb\nc\r\nd' | od -An -c | tr -s ' ')"
}

@test "util trailing-whitespace - spaces around a stray CR are fixed in one pass" {
    printf "x   \r \ny\n" > stray.txt

    run hk util trailing-whitespace --fix stray.txt
    assert_success

    run hk util trailing-whitespace stray.txt
    assert_success
    refute_output
    assert_equal "$(od -An -c stray.txt | tr -s ' ')" "$(printf 'x\r\ny\n' | od -An -c | tr -s ' ')"
}

@test "util trailing-whitespace - only strips spaces and tabs" {
    printf "form\fgap\f\nlone cr\r" > odd.txt

    run hk util trailing-whitespace odd.txt
    assert_success
    refute_output
}
