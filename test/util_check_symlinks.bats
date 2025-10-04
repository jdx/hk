#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util check-symlinks - detects broken symlink" {
    ln -s nonexistent broken_link

    run hk util check-symlinks broken_link
    assert_failure
    assert_output --partial "broken_link"
}

@test "util check-symlinks - passes valid symlink" {
    echo "content" > target.txt
    ln -s target.txt link

    run hk util check-symlinks link
    assert_success
    refute_output
}

@test "util check-symlinks - passes regular file" {
    echo "content" > file.txt

    run hk util check-symlinks file.txt
    assert_success
    refute_output
}

@test "util check-symlinks - detects multiple broken symlinks" {
    ln -s nonexistent1 broken1
    ln -s nonexistent2 broken2

    run hk util check-symlinks broken1 broken2
    assert_failure
    assert_output --partial "broken1"
    assert_output --partial "broken2"
}

@test "util check-symlinks - mixed valid and broken" {
    echo "content" > target.txt
    ln -s target.txt valid_link
    ln -s nonexistent broken_link

    run hk util check-symlinks valid_link broken_link
    assert_failure
    assert_output --partial "broken_link"
    refute_output --partial "valid_link"
}

@test "util check-symlinks - symlink to directory" {
    mkdir target_dir
    ln -s target_dir link_to_dir

    run hk util check-symlinks link_to_dir
    assert_success
    refute_output
}

@test "util check-symlinks - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["symlinks"] = Builtins.check_symlinks
        }
    }
}
HK

    ln -s nonexistent broken_link
    git add -A

    run hk check --all
    assert_failure
    assert_output --partial "broken_link"
}
