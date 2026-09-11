#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "util destroyed-symlinks - detects symlink replaced by its target path" {
    echo "content" > target.txt
    ln -s target.txt link
    git add -A
    git commit -qm initial

    rm link
    printf 'target.txt' > link
    git add link

    run hk util destroyed-symlinks
    assert_failure
    assert_output --partial "link"
    assert_output --partial "git reset HEAD --"
}

@test "util destroyed-symlinks - passes intact symlink" {
    echo "content" > target.txt
    ln -s target.txt link
    git add -A
    git commit -qm initial

    run hk util destroyed-symlinks
    assert_success
    refute_output
}

@test "util destroyed-symlinks - passes edited regular file" {
    echo "content" > target.txt
    git add -A
    git commit -qm initial

    echo "changed" > target.txt
    git add target.txt

    run hk util destroyed-symlinks
    assert_success
    refute_output
}

@test "util destroyed-symlinks - passes deleted symlink" {
    echo "content" > target.txt
    ln -s target.txt link
    git add -A
    git commit -qm initial

    git rm -q link

    run hk util destroyed-symlinks
    assert_success
    refute_output
}

@test "util destroyed-symlinks - tolerates a trailing newline added by a formatter" {
    echo "content" > target.txt
    ln -s target.txt link
    git add -A
    git commit -qm initial

    rm link
    printf 'target.txt\n' > link
    git add link

    run hk util destroyed-symlinks
    assert_failure
    assert_output --partial "link"
}

@test "util destroyed-symlinks - scopes to the supplied paths" {
    echo "content" > target.txt
    mkdir -p other
    ln -s ../target.txt other/link
    git add -A
    git commit -qm initial

    rm other/link
    printf '../target.txt' > other/link
    git add other/link

    run hk util destroyed-symlinks target.txt
    assert_success
    refute_output

    run hk util destroyed-symlinks other/link
    assert_failure
    assert_output --partial "other/link"
}

@test "util destroyed-symlinks - builtin integration" {
    cat > hk.pkl <<HK
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["destroyed-symlinks"] = Builtins.destroyed_symlinks
        }
    }
}
HK

    echo "content" > target.txt
    ln -s target.txt link
    git add -A
    git commit -qm initial

    rm link
    printf 'target.txt' > link
    git add link

    run hk check --all
    assert_failure
    assert_output --partial "link"
}
