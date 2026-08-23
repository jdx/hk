#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "dir renders {{workspace}} so a step runs in each workspace" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["workspace-cwd"] {
                glob = List("**/*.go")
                workspace_indicator = "go.mod"
                dir = "{{workspace}}"
                check = "pwd"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p pkgs/api pkgs/worker
    echo "module example.com/api" > pkgs/api/go.mod
    echo "package api" > pkgs/api/main.go
    echo "module example.com/worker" > pkgs/worker/go.mod
    echo "package worker" > pkgs/worker/main.go
    git add pkgs

    run hk check -v
    assert_success

    # One job per workspace, each running in the workspace directory.
    assert_output --partial "pkgs/api"
    assert_output --partial "pkgs/worker"
}

@test "templated dir makes {{files}} relative to the rendered directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["workspace-files"] {
                glob = List("**/*.go")
                workspace_indicator = "go.mod"
                dir = "{{workspace}}"
                check = "echo 'checking {{files}}' && test -f {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p pkgs/api/internal
    echo "module example.com/api" > pkgs/api/go.mod
    echo "package internal" > pkgs/api/internal/svc.go
    git add pkgs

    run hk check -v
    assert_success
    assert_output --partial "checking internal/svc.go"
}

@test "templated dir does not filter files by the literal template text" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["workspace-glob"] {
                glob = List("**/*.go")
                workspace_indicator = "go.mod"
                dir = "{{workspace}}"
                check = "echo 'ran in {{workspace}}'"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p pkgs/api
    echo "module example.com/api" > pkgs/api/go.mod
    echo "package api" > pkgs/api/main.go
    git add pkgs

    run hk check -v
    assert_success
    # A literal "{{workspace}}" prefix would match no file and skip the step.
    refute_output --partial "no files to process"
    assert_output --partial "ran in pkgs/api"
}

@test "the literal prefix of a templated dir still scopes file selection" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["api"] {
                dir = "sub/{{step}}"
                glob = List("**/*.txt")
                check = "echo 'found {{files}}'"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p sub/api other
    echo "a" > sub/api/a.txt
    echo "b" > other/b.txt
    echo "c" > root.txt
    git add sub other root.txt

    run hk check -v
    assert_success

    # "sub" is the literal prefix of the templated dir, so selection stays
    # scoped to it, and paths are relative to the rendered "sub/api".
    assert_output --partial "found a.txt"
    refute_output --partial "found other/b.txt"
    refute_output --partial "found root.txt"
}

@test "a leading ./ in dir does not break file selection or {{files}}" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["dot-slash-literal"] {
                dir = "./sub"
                glob = List("**/*.txt")
                check = "echo 'literal {{files}}' && test -f {{files}}"
            }
            ["dot-slash-template"] {
                dir = "./sub/{{step}}"
                glob = List("**/*.md")
                check = "echo 'templated {{files}}' && test -f {{files}}"
            }
            ["repo-root"] {
                dir = "."
                glob = List("**/*.txt")
                check = "echo 'root {{files}}' && test -f {{files}}"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p "sub/dot-slash-template"
    echo "a" > sub/a.txt
    echo "b" > sub/dot-slash-template/b.md
    git add sub

    run hk check -v
    assert_success

    # "./sub" must behave exactly like "sub": the file is selected and the path
    # is made relative to it, not left repo-root-relative and doubled.
    assert_output --partial "literal a.txt"
    assert_output --partial "templated b.md"
    # "." is the repo root, so it narrows nothing and paths stay as they are.
    assert_output --partial "root sub/a.txt"
}

@test "a dir that renders to a missing directory names the step and the path" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["missing"] {
                glob = List("**/*.txt")
                dir = "nope/{{step}}"
                check = "true"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p nope
    echo "a" > nope/a.txt
    git add nope

    run hk check -v
    assert_failure
    # Without this the spawn fails with a bare "No such file or directory".
    assert_output --partial "missing: working directory does not exist: nope/missing"
}

@test "stage patterns are scoped to each workspace when dir is templated" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["fix"] {
        fix = true
        steps {
            ["gen"] {
                glob = List("**/*.txt")
                workspace_indicator = "marker"
                dir = "{{workspace}}"
                stage = List("generated/**")
                fix = "mkdir -p generated && echo out > generated/o.txt"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    mkdir -p pkgs/a pkgs/b generated
    touch pkgs/a/marker pkgs/b/marker
    echo "x" > pkgs/a/in.txt
    echo "x" > pkgs/b/in.txt
    echo "unrelated" > generated/root.txt
    git add pkgs
    git commit -m "add workspaces"
    echo "y" >> pkgs/a/in.txt
    echo "y" >> pkgs/b/in.txt
    git add pkgs/a/in.txt pkgs/b/in.txt

    run hk fix
    assert_success

    run git diff --cached --name-only
    # The pattern is resolved once per workspace, so both generated files are
    # staged and the same-named path at the repo root is left alone.
    assert_line "pkgs/a/generated/o.txt"
    assert_line "pkgs/b/generated/o.txt"
    refute_line "generated/root.txt"
}

@test "a dir that renders to a file says so instead of reporting it missing" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["notadir"] {
                glob = List("**/*.txt")
                dir = "{{step}}"
                check = "true"
            }
        }
    }
}
EOF
    git add hk.pkl
    git commit -m "initial commit"

    echo "i am a file" > notadir
    echo "a" > a.txt
    git add notadir a.txt

    run hk check -v
    assert_failure
    assert_output --partial "notadir: working directory is not a directory: notadir"
}
