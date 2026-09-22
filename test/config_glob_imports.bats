#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

_write_glob_config() {
    mkdir -p generated
    cat <<EOF > generated/one.pkl
glob = List("*.js")
check = "echo one {{files}}"
EOF
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

local generated = import*("generated/*.pkl")

hooks {
    ["check"] {
        steps {
            for (path, step in generated) {
                [path.split("/").last.split(".").first] = step
            }
        }
    }
}
EOF
    echo "test" > test.js
    git init
    git add .
    git commit -m "initial commit"
}

@test "glob import expression defines steps" {
    _write_glob_config
    run hk check --all
    assert_success
    assert_output --partial "one test.js"
}

@test "glob import declaration defines steps" {
    mkdir -p generated
    cat <<EOF > generated/one.pkl
glob = List("*.js")
check = "echo one {{files}}"
EOF
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

import* "generated/*.pkl" as Generated

hooks {
    ["check"] {
        steps {
            for (path, step in Generated) {
                [path.split("/").last.split(".").first] = step
            }
        }
    }
}
EOF
    echo "test" > test.js
    git init
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "one test.js"
}

@test "editing a glob-imported file invalidates the config cache" {
    _write_glob_config
    run hk check --all
    assert_success
    assert_output --partial "one test.js"

    cat <<EOF > generated/one.pkl
glob = List("*.js")
check = "echo edited {{files}}"
EOF
    run hk check --all
    assert_success
    assert_output --partial "edited test.js"
}

@test "glob import matching nothing leaves the hook empty" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"

local generated = import*("generated/*.pkl")

hooks {
    ["check"] {
        steps {
            for (path, step in generated) {
                [path.split("/").last.split(".").first] = step
            }
        }
    }
}
EOF
    echo "test" > test.js
    git init
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    refute_output --partial "one test.js"
}
