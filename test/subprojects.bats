#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "subprojects merges nested configs scoped to their directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("sub", "packages/*")
hooks {
    ["check"] {}
}
EOF
    mkdir -p sub packages/a packages/b
    cat <<EOF > sub/hk.pkl
amends "$PKL_PATH/Config.pkl"
env {
    ["GREETING"] = "hello-from-sub"
}
hooks {
    ["check"] {
        steps {
            ["greet"] {
                glob = "*.txt"
                check = "echo GREETING=\$GREETING; for f in {{files}}; do echo checked \$f; done; exit 1"
            }
        }
    }
}
EOF
    cat <<EOF > packages/a/hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["pkga"] {
                glob = "*.txt"
                check = "echo pkga saw {{files}}; exit 1"
            }
        }
    }
}
EOF
    echo "root" > root.txt
    echo "sub" > sub/ok.txt
    echo "a" > packages/a/a.txt
    echo "b" > packages/b/b.txt
    git add .
    git commit -m "initial commit"

    run hk check --all --no-fail-fast -v
    assert_failure
    # scoped step names
    assert_output --partial "sub:greet"
    assert_output --partial "packages/a:pkga"
    # subproject env applies to its steps
    assert_output --partial "GREETING=hello-from-sub"
    # files are scoped to the subproject dir (and relative to it)
    assert_output --partial "checked ok.txt"
    assert_output --partial "pkga saw a.txt"
    refute_output --partial "checked root.txt"
    refute_output --partial "checked ../root.txt"
}

@test "subprojects glob skips directories without an hk config" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("packages/*")
hooks {
    ["check"] {}
}
EOF
    mkdir -p packages/a packages/b
    cat <<EOF > packages/a/hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["ok"] {
                glob = "*.txt"
                check = "true"
            }
        }
    }
}
EOF
    echo "a" > packages/a/a.txt
    echo "b" > packages/b/b.txt
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
}

@test "subprojects warns on missing literal directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("does-not-exist")
hooks {
    ["check"] {}
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "subprojects: directory not found: does-not-exist"
}

@test "subprojects work in git hooks from the repo root" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("sub")
hooks {
    ["pre-commit"] {}
}
EOF
    mkdir -p sub
    cat <<EOF > sub/hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["no-todo"] {
                glob = "*.txt"
                check = "! grep -H TODO {{files}}"
            }
        }
    }
}
EOF
    git add .
    git commit -m "initial commit"
    hk install

    echo "TODO fixme" > sub/bad.txt
    git add sub/bad.txt
    run git commit -m "should fail"
    assert_failure
    assert_output --partial "sub:no-todo"

    echo "all good" > sub/bad.txt
    git add sub/bad.txt
    run git commit -m "should pass"
    assert_success
}
