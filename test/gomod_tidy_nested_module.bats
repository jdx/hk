#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

# gomod_tidy runs `go mod tidy`, which resolves the module rooted at the working
# directory. Without workspace_indicator/dir it ran at the repo root and failed
# outright for any module living in a subdirectory.
@test "gomod_tidy tidies a module in a subdirectory" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["gomod_tidy"] = Builtins.gomod_tidy()
    }
  }
  ["fix"] {
    steps {
      ["gomod_tidy"] = Builtins.gomod_tidy()
    }
  }
}
PKL

    mkdir -p svc
    printf 'module example.com/svc\n\ngo 1.21\n\nrequire example.com/unused v1.0.0\n' > svc/go.mod
    printf 'package main\n\nfunc main() {}\n' > svc/main.go

    git add -A
    git commit -m "init" --quiet

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"

    # check must report the untidy module rather than failing to find it.
    # `go mod tidy -diff` labels its output current/go.mod and tidy/go.mod, so
    # the module path is what proves the diff came from svc/ and not the root.
    run hk check --all --step gomod_tidy
    assert_failure
    refute_output --partial "go.mod file not found"
    assert_output --partial "module example.com/svc"
    assert_output --partial "-require example.com/unused v1.0.0"

    run hk fix --all --step gomod_tidy
    assert_success

    run cat svc/go.mod
    assert_success
    assert_output --partial "module example.com/svc"
    refute_output --partial "example.com/unused"
}

@test "gomod_tidy runs on Go changes and stages updated manifests" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["fix"] {
    steps {
      ["gomod_tidy"] = Builtins.gomod_tidy()
    }
  }
}
PKL

    mkdir -p svc
    printf 'module example.com/svc\n\ngo 1.21\n\nrequire example.com/unused v1.0.0\n' > svc/go.mod
    printf 'package main\n\nfunc main() {}\n' > svc/main.go
    git add -A
    git commit -m "init" --quiet

    printf 'package main\n\nfunc main() { println("changed") }\n' > svc/main.go
    git add svc/main.go

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk fix --step gomod_tidy
    assert_success

    run git diff --cached --name-only
    assert_success
    assert_line "svc/main.go"
    assert_line "svc/go.mod"

    run git diff -- svc/go.mod
    assert_success
    refute_output
}
