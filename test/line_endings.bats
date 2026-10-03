#!/usr/bin/env bats

# trailing-whitespace, end-of-file-fixer, and mixed-line-ending each preserve a
# file's line terminators, so running all of them, in any order, leaves a CRLF
# file all CRLF and a second run changes nothing.

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# Print a file's bytes in a form that can be compared as text
bytes() {
    od -An -c "$1" | tr -s ' '
}

@test "line endings - all three fixers leave a CRLF file all CRLF in any order" {
    local orders=(
        "trailing-whitespace end-of-file-fixer mixed-line-ending"
        "trailing-whitespace mixed-line-ending end-of-file-fixer"
        "end-of-file-fixer trailing-whitespace mixed-line-ending"
        "end-of-file-fixer mixed-line-ending trailing-whitespace"
        "mixed-line-ending trailing-whitespace end-of-file-fixer"
        "mixed-line-ending end-of-file-fixer trailing-whitespace"
    )
    local expected
    expected="$(printf 'one\r\ntwo\r\nthree\r\n' | od -An -c | tr -s ' ')"
    for order in "${orders[@]}"; do
        printf 'one  \r\ntwo\t\r\nthree\r\n\r\n\r\n' > file.txt
        for cmd in $order; do
            hk util "$cmd" --fix file.txt
        done
        assert_equal "$(bytes file.txt)" "$expected"

        # A second pass finds nothing to do
        for cmd in $order; do
            run hk util "$cmd" file.txt
            assert_success
            refute_output
        done
    done
}

@test "line endings - CRLF file without a final newline stays all CRLF" {
    # newlines used to append a bare LF here, which mixed_line_ending then
    # flattened into LF for the whole file.
    printf 'one\r\ntwo\r\nthree' > file.txt

    hk util end-of-file-fixer --fix file.txt
    run hk util mixed-line-ending file.txt
    assert_success
    refute_output
    assert_equal "$(bytes file.txt)" "$(printf 'one\r\ntwo\r\nthree\r\n' | od -An -c | tr -s ' ')"
}

@test "line endings - builtins together fix a CRLF file and then pass check" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["trailing_whitespace"] = Builtins.trailing_whitespace
            ["newlines"] = Builtins.newlines
            ["mixed_line_ending"] = Builtins.mixed_line_ending
        }
    }
    ["fix"] {
        steps {
            ["trailing_whitespace"] = Builtins.trailing_whitespace
            ["newlines"] = Builtins.newlines
            ["mixed_line_ending"] = Builtins.mixed_line_ending
        }
    }
}
HK
    git init -q .
    printf 'one  \r\ntwo\r\nthree' > test.txt
    printf 'a\r\nb  \r\n\r\n' > other.txt
    printf 'unix  \nlines\n' > lf.txt
    git add -A

    run hk check
    assert_failure

    run hk fix
    assert_success

    assert_equal "$(bytes test.txt)" "$(printf 'one\r\ntwo\r\nthree\r\n' | od -An -c | tr -s ' ')"
    assert_equal "$(bytes other.txt)" "$(printf 'a\r\nb\r\n' | od -An -c | tr -s ' ')"
    assert_equal "$(bytes lf.txt)" "$(printf 'unix\nlines\n' | od -An -c | tr -s ' ')"

    run hk check
    assert_success
}

@test "line endings - trailing-whitespace CRLF diff applies with git apply" {
    git init -q .
    printf "contents  \r\nmore  \r\nlast" > crlf.txt

    hk util trailing-whitespace --diff crlf.txt > fix.patch || true
    git apply --check fix.patch
    git apply fix.patch
    assert_equal "$(od -An -c crlf.txt | tr -s ' ')" "$(printf 'contents\r\nmore\r\nlast' | od -An -c | tr -s ' ')"
}

@test "line endings - end-of-file-fixer CRLF diff applies with git apply" {
    git init -q .
    printf "one\r\ntwo\r\n\r\n" > blank.txt
    printf "one\r\ntwo" > missing.txt

    hk util end-of-file-fixer --diff blank.txt missing.txt > fix.patch || true
    git apply --check fix.patch
    git apply fix.patch
    assert_equal "$(od -An -c blank.txt | tr -s ' ')" "$(printf 'one\r\ntwo\r\n' | od -An -c | tr -s ' ')"
    assert_equal "$(od -An -c missing.txt | tr -s ' ')" "$(printf 'one\r\ntwo\r\n' | od -An -c | tr -s ' ')"
}

@test "line endings - mixed-line-ending diff applies with git apply" {
    git init -q .
    printf "line1\r\nline2\nline3\r\n" > crlf.txt
    printf "line1\nline2\r\nline3\n" > lf.txt

    hk util mixed-line-ending --diff crlf.txt lf.txt > fix.patch || true
    git apply --check fix.patch
    git apply fix.patch
    assert_equal "$(od -An -c crlf.txt | tr -s ' ')" "$(printf 'line1\r\nline2\r\nline3\r\n' | od -An -c | tr -s ' ')"
    assert_equal "$(od -An -c lf.txt | tr -s ' ')" "$(printf 'line1\nline2\nline3\n' | od -An -c | tr -s ' ')"
}
