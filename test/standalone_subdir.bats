#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "a config found by walking up from a subdirectory scopes itself, not the whole repo" {
    mkdir -p sub
    cat <<EOF > sub/hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["list"] {
                glob = "*.txt"
                check = "for f in {{files}}; do echo checked \$f; done"
            }
        }
    }
}
EOF
    echo "root" > root.txt
    echo "sub" > sub/ok.txt
    git add .
    git commit -m "initial commit"

    cd sub
    run hk check --all
    assert_success
    assert_output --partial "checked ok.txt"
    refute_output --partial "checked root.txt"
    refute_output --partial "checked ../root.txt"
}

@test "a nested subprojects entry inside a standalone-loaded config is scoped to the work tree root" {
    mkdir -p sub/api
    cat <<EOF > sub/hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("api")
hooks {
    ["check"] {
        steps {
            ["list"] {
                glob = "*.txt"
                check = "for f in {{files}}; do echo checked \$f; done"
            }
        }
    }
}
EOF
    cat <<EOF > sub/api/hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["list"] {
                glob = "*.txt"
                check = "for f in {{files}}; do echo checked \$f; done"
            }
        }
    }
}
EOF
    echo "root" > root.txt
    echo "sub" > sub/ok.txt
    echo "api" > sub/api/ok.txt
    git add .
    git commit -m "initial commit"

    cd sub
    run hk check --all
    assert_success
    assert_output --partial "api:list"
    assert_output --partial "api:list – checked ok.txt"
    refute_output --partial "checked root.txt"
}
