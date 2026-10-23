#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "check" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git init
    git add .
    git commit -m "initial commit"
    echo "" >> hk.pkl
    echo "test" > test.js
    run hk check
    assert_success
    assert_output --partial "checking hk.pkl test.js"
}

@test "check files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    echo "test" > test.js
    run hk check test.js
    assert_success
    assert_output --partial "checking test.js"
}

@test "check files dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    mkdir -p a/b/c
    echo "test" > a/b/c/test.js
    run hk check a
    assert_success
    assert_output --partial "checking a/b/c/test.js"
}

@test "check files w/ exclude" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    echo "test" > test.js
    run hk check hk.pkl test.js --exclude hk.pkl
    assert_success
    assert_output --partial "checking test.js"
}

@test "check files w/ exclude dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    mkdir -p a/b/c a/d
    echo "test" > a/b/c/test.js
    echo "test" > a/d/test.js
    run hk check a --exclude a/b
    assert_success
    assert_output --partial "checking a/d/test.js"
}

@test "check glob" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    echo "test" > test.js
    echo "test" > test.ts
    git init
    git add .
    git commit -m "initial commit"
    run hk check --glob "*.js"
    assert_success
    assert_output --partial "checking test.js"
}

@test "check glob dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git init
    git add .
    git commit -m "initial commit"
    mkdir -p a/b/c a/d d
    echo "test" > a/b/c/test.js
    echo "test" > a/d/test.js
    echo "test" > d/test.ts
    run hk check a --glob "*.js"
    assert_success
    assert_output --partial "checking a/b/c/test.js a/d/test.js"
}

@test "check glob exclude" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    echo "test" > test.js
    echo "test" > test.ts
    git init
    git add .
    git commit -m "initial commit"
    run hk check --all --exclude "*.ts" --exclude "hk.pkl"
    assert_success
    assert_output --partial "checking test.js"
}

@test "check files resolves relative to the invocation directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git init
    mkdir sub
    echo "root" > a.js
    echo "sub" > sub/a.js
    git add .
    git commit -m "initial commit"
    cd sub
    run hk check a.js
    assert_success
    assert_output --partial "checking sub/a.js"
    refute_output --partial "checking a.js"
    run hk check ../a.js
    assert_success
    assert_output --partial "checking a.js"
    run hk check .
    assert_success
    assert_output --partial "checking sub/a.js"
}

@test "check files follows directory symlinks before .." {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git init
    mkdir -p nested/child
    echo "root" > a.js
    echo "nested" > nested/a.js
    ln -s nested/child link
    git add .
    git commit -m "initial commit"
    run hk check link/../a.js
    assert_success
    assert_output --partial "checking nested/a.js"
}

@test "check files keeps unresolvable .. paths unchanged" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { check = "echo checking {{files}}" } } }
}
EOF
    git init
    echo "root" > a.js
    git add .
    git commit -m "initial commit"
    run hk check missing/../a.js
    assert_success
    refute_output --partial "checking a.js"
}

@test "check files resolves directory arguments ending in .." {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { dir = "src"; check = "echo checking {{files}}" } } }
}
EOF
    git init
    mkdir -p src sub
    echo "x" > src/a.js
    echo "y" > sub/b.js
    git add .
    git commit -m "initial commit"
    run hk check sub/..
    assert_success
    assert_output --partial "checking a.js"
}

@test "check files keeps a final directory symlink name after .." {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { dir = "alias"; check = "echo checking {{files}}" } } }
}
EOF
    git init
    mkdir -p sub real
    echo "x" > real/a.js
    ln -s real alias
    git add .
    git commit -m "initial commit"
    HK_WALK_IGNORE=0 run hk check sub/../alias
    assert_success
    assert_output --partial "checking a.js"
}

@test "check files with a symlinked GIT_WORK_TREE stays repo-relative" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] { steps { ["a"] { dir = "src"; check = "echo checking {{files}}" } } }
}
EOF
    git init
    mkdir -p src sub
    echo "x" > src/a.js
    git add .
    git commit -m "initial commit"
    ln -s "$PWD" ../tree-link
    export GIT_DIR="$PWD/.git" GIT_WORK_TREE="$PWD/../tree-link"
    cd sub
    run hk check ../src/a.js
    assert_success
    assert_output --partial "checking a.js"
}
