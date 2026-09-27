#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    # A "formatter" that upper-cases stdin.
    cat <<'SCRIPT' > upper.sh
#!/bin/sh
tr a-z A-Z
SCRIPT
    chmod +x upper.sh
}
teardown() {
    _common_teardown
}

@test "util format-diff prints a patch that git apply accepts" {
    printf 'hello\n' > a.txt
    printf 'SAME\n' > b.txt

    run --separate-stderr hk util format-diff a.txt b.txt -- ./upper.sh
    assert_failure 1
    assert_output $'--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-hello\n+HELLO'

    echo "$output" | git apply -p0
    run cat a.txt
    assert_output "HELLO"
}

@test "util format-diff exits 0 with no output when nothing would change" {
    printf 'SAME\n' > a.txt

    run hk util format-diff a.txt -- ./upper.sh
    assert_success
    refute_output
}

@test "util format-diff replaces {} with the file's path" {
    cat <<'SCRIPT' > path.sh
#!/bin/sh
cat
echo "formatted as $1"
SCRIPT
    chmod +x path.sh
    printf 'x\n' > a.txt

    run hk util format-diff a.txt -- ./path.sh {}
    assert_failure 1
    assert_output --partial "+formatted as a.txt"
}

@test "util format-diff prints no patch when the formatter fails for any file" {
    cat <<'SCRIPT' > picky.sh
#!/bin/sh
input=$(cat)
case $input in
*broken*) echo "cannot parse" >&2; exit 3 ;;
esac
printf '%s\n' "$input" | tr a-z A-Z
SCRIPT
    chmod +x picky.sh
    printf 'hello\n' > a.txt
    printf 'broken\n' > b.txt

    run --separate-stderr hk util format-diff a.txt b.txt -- ./picky.sh
    assert_failure 3
    refute_output
    [[ "$stderr" == *"b.txt"*"cannot parse"* ]]
}

@test "util format-diff treats empty output for a non-empty file as a failure" {
    printf 'hello\n' > a.txt

    run --separate-stderr hk util format-diff a.txt -- true
    assert_failure 1
    refute_output
    [[ "$stderr" == *"a.txt"*"printed nothing"* ]]
}

@test "a check_diff built with util format-diff is applied instead of running the fixer" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["upper"] {
                glob = List("*.txt")
                check_diff = "hk util format-diff {{files}} -- ./upper.sh"
                fix = "for f in {{files}}; do echo fixer-ran > \"\$f\"; done"
            }
        }
    }
}
EOF
    printf 'hello\n' > a.txt
    printf 'SAME\n' > b.txt

    run hk fix a.txt b.txt
    assert_success
    run cat a.txt
    assert_output "HELLO"
    run cat b.txt
    assert_output "SAME"
}

@test "util format-diff treats a formatter that stops reading early as a failure" {
    # It exits 0 after reading part of a large file, so its output is only
    # part of the formatted file.
    head -c 1048576 /dev/zero | tr '\0' 'a' > big.txt
    echo >> big.txt

    run --separate-stderr hk util format-diff big.txt -- sh -c 'head -c 5; exit 0'
    assert_failure
    refute_output
    [[ "$stderr" == *"big.txt: writing to sh"* ]]
}

@test "util format-diff --no-stdin runs a formatter that reads the file itself" {
    printf 'hello\n' > a.txt

    run --separate-stderr hk util format-diff --no-stdin a.txt -- sh -c 'tr a-z A-Z < "$1"' _ {}
    assert_failure 1
    assert_output $'--- a.txt\n+++ a.txt\n@@ -1 +1 @@\n-hello\n+HELLO'
}
