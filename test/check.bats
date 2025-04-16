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
    run hk check --all --exclude-glob "*.ts" --exclude-glob "hk.pkl"
    assert_success
    assert_output --partial "checking test.js"
}
