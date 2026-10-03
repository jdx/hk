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
steps {
    ["greet"] {
        glob = "*.txt"
        check = "echo GREETING=\$GREETING; for f in {{files}}; do echo checked \$f; done; exit 1"
    }
}
EOF
    cat <<EOF > packages/a/hk.pkl
amends "$PKL_PATH/Config.pkl"
steps {
    ["pkga"] {
        glob = "*.txt"
        check = "echo pkga saw {{files}}; exit 1"
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

@test "subprojects make workspace templates relative to the scoped directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("ui")
hooks {
    ["check"] {}
}
EOF
    mkdir -p ui/src
    cat <<EOF > ui/hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["tsc-paths"] {
                glob = List("**/*.ts")
                workspace_indicator = "tsconfig.json"
                check = "echo workspace={{workspace}} indicator={{workspace_indicator}}; test -f {{workspace_indicator}}; test '{{workspace}}' = '.'"
            }
        }
    }
}
EOF
    echo '{"compilerOptions":{"strict":true}}' > ui/tsconfig.json
    echo 'const value: number = 1;' > ui/src/main.ts
    git add .
    git commit -m "initial commit"

    run hk check --all -v
    assert_success
    assert_output --partial "workspace=. indicator=tsconfig.json"
}

@test "subproject skip_steps skips only that subproject's step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("a", "b")
hooks {
    ["check"] {}
}
EOF
    mkdir -p a b
    cat <<EOF > a/hk.pkl
amends "$PKL_PATH/Config.pkl"
skip_steps = List("lint", "no-such-step")
steps {
    ["lint"] { glob = "*.txt"; check = "echo LINT-A" }
    ["fmt"] { glob = "*.txt"; check = "echo FMT-A" }
}
EOF
    cat <<EOF > b/hk.pkl
amends "$PKL_PATH/Config.pkl"
steps {
    ["lint"] { glob = "*.txt"; check = "echo LINT-B" }
}
EOF
    echo a > a/a.txt
    echo b > b/b.txt
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    # `lint` is skipped in `a` only, and `fmt` in `a` still runs
    refute_output --partial "LINT-A"
    assert_output --partial "LINT-B"
    assert_output --partial "FMT-A"
    # A name that matches no step is reported rather than silently dropped
    assert_output --partial "skip_steps entry 'no-such-step'"
}

@test "subproject top-level settings that have no effect produce a warning" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("sub")
hooks {
    ["check"] {}
}
EOF
    mkdir -p sub
    cat <<EOF > sub/hk.pkl
amends "$PKL_PATH/Config.pkl"
exclude = List("*.txt")
fail_fast = false
steps {
    ["lint"] { glob = "*.txt"; check = "echo LINT-SUB {{files}}" }
}
EOF
    echo a > sub/a.txt
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    assert_output --partial "ignoring top-level exclude, fail_fast"
    assert_output --partial "sub/hk.pkl"
    # The subproject's exclude is not applied; only the root config's is
    assert_output --partial "LINT-SUB a.txt"
}

@test "a subproject config that sets only steps, hooks and env warns about nothing" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
subprojects = List("sub")
hooks {
    ["check"] {}
}
EOF
    mkdir -p sub
    cat <<EOF > sub/hk.pkl
amends "$PKL_PATH/Config.pkl"
env {
    ["GREETING"] = "hello"
}
steps {
    ["lint"] { glob = "*.txt"; check = "echo LINT-SUB" }
}
EOF
    echo a > sub/a.txt
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_success
    refute_output --partial "ignoring top-level"
}
