#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util check-merge-conflict - detects conflict markers" {
    cat > file1.txt <<EOF
normal line
<<<<<<< HEAD
my changes
=======
their changes
>>>>>>> branch
EOF

    run hk util check-merge-conflict file1.txt
    assert_failure
    assert_output "file1.txt"
}

@test "util check-merge-conflict - passes clean files" {
    echo "normal line" > file1.txt
    echo "another line" >> file1.txt

    run hk util check-merge-conflict file1.txt
    assert_success
    refute_output
}

@test "util check-merge-conflict - detects only start marker" {
    cat > file1.txt <<EOF
normal line
<<<<<<< HEAD
some changes
EOF

    run hk util check-merge-conflict file1.txt
    assert_failure
    assert_output "file1.txt"
}

@test "util check-merge-conflict - detects middle marker" {
    cat > file1.txt <<EOF
normal line
=======
some changes
EOF

    run hk util check-merge-conflict file1.txt
    assert_failure
    assert_output "file1.txt"
}

@test "util check-merge-conflict - detects end marker" {
    cat > file1.txt <<EOF
normal line
>>>>>>> branch
EOF

    run hk util check-merge-conflict file1.txt
    assert_failure
    assert_output "file1.txt"
}

@test "util check-merge-conflict - multiple files" {
    cat > file1.txt <<EOF
<<<<<<< HEAD
conflict
EOF
    cat > file2.txt <<EOF
=======
conflict
EOF

    run hk util check-merge-conflict file1.txt file2.txt
    assert_failure
    assert_output --partial "file1.txt"
    assert_output --partial "file2.txt"
}

@test "util check-merge-conflict - ignores markers in middle of line" {
    echo "this is not <<<<<<< a conflict" > file1.txt

    run hk util check-merge-conflict file1.txt
    assert_success
    refute_output
}

@test "util check-merge-conflict - detects with leading whitespace" {
    echo "  <<<<<<< HEAD  " > file1.txt

    run hk util check-merge-conflict file1.txt
    assert_failure
    assert_output "file1.txt"
}

@test "util check-merge-conflict - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["merge-conflict"] = Builtins.check_merge_conflict
        }
    }
}
HK

    cat > test.txt <<EOF
<<<<<<< HEAD
conflict
EOF

    run hk check
    assert_failure
    assert_output --partial "test.txt"
}
