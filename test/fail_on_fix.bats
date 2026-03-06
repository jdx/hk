#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "fail_on_fix=true fails when fixer modifies files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = false
        fail_on_fix = true
        steps {
            ["add-newline"] {
                glob = "*.txt"
                fix = #"for f in {{ files }}; do echo >> \$f; done"#
            }
        }
    }
}
EOF
    echo -n "no newline" > file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit"

    echo "modified" > file.txt

    run hk run fix
    assert_failure
}

@test "fail_on_fix=true passes when no files are modified" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = false
        fail_on_fix = true
        steps {
            ["noop"] {
                glob = "*.txt"
                fix = "true"
            }
        }
    }
}
EOF
    echo "content" > file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit"

    hk run fix
}

@test "fail_on_fix=false (default) passes when fixer modifies files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        stage = false
        steps {
            ["add-newline"] {
                glob = "*.txt"
                fix = #"for f in {{ files }}; do echo >> \$f; done"#
            }
        }
    }
}
EOF
    echo -n "no newline" > file.txt
    git add hk.pkl file.txt
    git commit -m "initial commit"

    echo "modified" > file.txt

    hk run fix
}
