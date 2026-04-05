#!/usr/bin/env bats

# Tests that Go package-level analysis builtins use workspace_indicator
# and ./... instead of {{ files }}, so they work with multi-package projects.

setup() {
  load 'test_helper/common_setup'
  _common_setup
  _go_builtins_setup
}

teardown() {
  _common_teardown
}

_go_builtins_setup() {
  # Simulate a Go project with multiple packages
  mkdir -p internal/a internal/b

  cat > go.mod <<'EOF'
module example.com/repro

go 1.21
EOF

  cat > main.go <<'EOF'
package main

func main() {}
EOF

  cat > internal/a/a.go <<'EOF'
package a

func Hello() string { return "hello" }
EOF

  cat > internal/b/b.go <<'EOF'
package b

func World() string { return "world" }
EOF

  git add -A
}

@test "go_vet builtin uses workspace_indicator and ./..." {
  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["go-vet"] = (Builtins.go_vet) {
        check = "echo {{workspace}} ./..."
      }
    }
  }
}
EOF
  git add hk.pkl

  run hk check --all -v
  assert_success
  # workspace should resolve to "." for a single-module project
  assert_output --partial "echo . ./..."
}

@test "go_vet builtin splits multi-workspace projects" {
  # Add a second go.mod to simulate a monorepo
  cat > internal/a/go.mod <<'EOF'
module example.com/repro/internal/a

go 1.21
EOF

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["go-vet"] = (Builtins.go_vet) {
        check = "echo ws={{workspace}}"
      }
    }
  }
}
EOF
  git add -A

  run hk check --all -v
  assert_success
  # Should see separate workspace invocations
  assert_output --partial "echo ws=."
  assert_output --partial "echo ws=internal/a"
}

@test "golangci_lint builtin uses workspace_indicator and ./..." {
  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["golangci-lint"] = (Builtins.golangci_lint) {
        check = "echo {{workspace}} ./..."
      }
    }
  }
}
EOF
  git add hk.pkl

  run hk check --all -v
  assert_success
  assert_output --partial "echo . ./..."
}

@test "staticcheck builtin uses workspace_indicator and ./..." {
  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
  ["check"] {
    steps {
      ["staticcheck"] = (Builtins.staticcheck) {
        check = "echo {{workspace}} ./..."
      }
    }
  }
}
EOF
  git add hk.pkl

  run hk check --all -v
  assert_success
  assert_output --partial "echo . ./..."
}
