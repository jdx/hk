#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util check-case-conflict - detects simple conflict" {
    if [[ "$(uname)" == "Darwin" ]]; then
        skip "macOS has case-insensitive filesystem by default"
    fi

    echo "first" > README.md
    echo "second" > readme.md

    run hk util check-case-conflict README.md readme.md
    assert_failure
    assert_output --partial "README.md"
    assert_output --partial "readme.md"
}

@test "util check-case-conflict - passes when no conflicts" {
    echo "content" > file1.txt
    echo "content" > file2.txt

    run hk util check-case-conflict file1.txt file2.txt
    assert_success
    refute_output
}

@test "util check-case-conflict - detects multiple conflicts" {
    if [[ "$(uname)" == "Darwin" ]]; then
        skip "macOS has case-insensitive filesystem by default"
    fi

    echo "1" > File1.txt
    echo "2" > file1.txt
    echo "3" > FILE1.TXT
    echo "a" > Other.md
    echo "b" > other.md

    run hk util check-case-conflict File1.txt file1.txt FILE1.TXT Other.md other.md
    assert_failure
    assert_output --partial "File1.txt"
    assert_output --partial "file1.txt"
    assert_output --partial "FILE1.TXT"
    assert_output --partial "Other.md"
    assert_output --partial "other.md"
}

@test "util check-case-conflict - detects conflict with different extensions" {
    if [[ "$(uname)" == "Darwin" ]]; then
        skip "macOS has case-insensitive filesystem by default"
    fi

    echo "text" > File.txt
    echo "text" > file.TXT

    run hk util check-case-conflict File.txt file.TXT
    assert_failure
    assert_output --partial "File.txt"
    assert_output --partial "file.TXT"
}

@test "util check-case-conflict - no conflict in different directories" {
    mkdir -p dir1 dir2
    echo "content" > dir1/file.txt
    echo "content" > dir2/file.txt

    run hk util check-case-conflict dir1/file.txt dir2/file.txt
    assert_success
    refute_output
}

@test "util check-case-conflict - detects conflict in subdirectory" {
    if [[ "$(uname)" == "Darwin" ]]; then
        skip "macOS has case-insensitive filesystem by default"
    fi

    mkdir -p src
    echo "main" > src/Main.rs
    echo "main" > src/main.rs

    run hk util check-case-conflict src/Main.rs src/main.rs
    assert_failure
    assert_output --partial "src/Main.rs"
    assert_output --partial "src/main.rs"
}

@test "util check-case-conflict - detects conflict with committed file" {
    if [[ "$(uname)" == "Darwin" ]]; then
        skip "macOS has case-insensitive filesystem by default"
    fi

    # Commit README.md
    echo "original" > README.md
    git add README.md
    git commit -m "Add README"

    # Try to add readme.md (should conflict with committed README.md)
    echo "new" > readme.md

    run hk util check-case-conflict readme.md
    assert_failure
    assert_output --partial "README.md"
    assert_output --partial "readme.md"
}

@test "util check-case-conflict - builtin integration" {
    if [[ "$(uname)" == "Darwin" ]]; then
        skip "macOS has case-insensitive filesystem by default"
    fi

    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["case-conflict"] = Builtins.check_case_conflict
        }
    }
}
HK

    echo "first" > README.md
    echo "second" > readme.md

    run hk check
    assert_failure
    assert_output --partial "README.md"
    assert_output --partial "readme.md"
}

@test "util check-case-conflict - builtin detects conflict with existing committed file" {
    if [[ "$(uname)" == "Darwin" ]]; then
        skip "macOS has case-insensitive filesystem by default"
    fi

    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["case-conflict"] = Builtins.check_case_conflict
        }
    }
}
HK

    # Commit README.md
    echo "original" > README.md
    git add README.md hk.pkl
    git commit -m "Add README"

    # Try to commit readme.md (should conflict)
    echo "new" > readme.md
    git add readme.md

    run hk run pre-commit
    assert_failure
    assert_output --partial "README.md"
    assert_output --partial "readme.md"
}

