#!/usr/bin/env bats

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

setup_mise_per_step_repo() {
  mkdir -p "$TEST_TEMP_DIR/bin" packages/client/tools packages/server

  cat > "$TEST_TEMP_DIR/bin/mise" <<'EOF'
#!/bin/sh
if [ "$1" != "x" ] || [ "$2" != "--" ]; then
  echo "unexpected mise args: $*" >&2
  exit 2
fi
shift 2
if [ -x "tools/nested-tool" ]; then
  PATH="$(pwd)/tools:$PATH"
  HK_TEST_WORKSPACE="client"
  export PATH HK_TEST_WORKSPACE
elif [ -f ".mise.toml" ]; then
  HK_TEST_WORKSPACE="server"
  export HK_TEST_WORKSPACE
fi
exec "$@"
EOF
  chmod +x "$TEST_TEMP_DIR/bin/mise"

  cat > packages/client/tools/nested-tool <<'EOF'
#!/bin/sh
printf 'nested-tool: pwd=%s args=%s env=%s\n' "$(pwd)" "$*" "$HK_TEST_WORKSPACE"
EOF
  chmod +x packages/client/tools/nested-tool

  PATH="$TEST_TEMP_DIR/bin:$PATH"

  touch mise.toml packages/client/mise.toml packages/server/.mise.toml
  printf 'bad\n' > packages/client/main.go
  printf 'server\n' > packages/server/main.go
  git add .
}

@test "HK_MISE_PER_STEP wraps multiline scripts in workspace mise env" {
  setup_mise_per_step_repo

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["nested-tool"] {
        glob = "packages/client/*.go"
        check = "nested-tool {{files}} && nested-tool {{files}}"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk check --all --step nested-tool -v
  assert_success
  assert_output --partial "nested-tool: pwd="
  assert_output --partial "packages/client"
  assert_output --partial "args=main.go env=client"
}

@test "HK_MISE_PER_STEP check_list_files output may be workspace-relative" {
  setup_mise_per_step_repo

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["list"] {
        glob = "packages/client/*.go"
        check_list_files = "echo main.go; exit 1"
        fix = "nested-tool {{files}}"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk fix --all --step list -v
  assert_success
  assert_output --partial "nested-tool:"
  assert_output --partial "args=main.go env=client"
  refute_output --partial "file in check output not found"
}

@test "HK_MISE_PER_STEP check_list_files output may be repo-relative" {
  setup_mise_per_step_repo

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["list"] {
        glob = "packages/client/*.go"
        check_list_files = "echo packages/client/main.go; exit 1"
        fix = "nested-tool {{files}}"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk fix --all --step list -v
  assert_success
  assert_output --partial "nested-tool:"
  assert_output --partial "args=main.go env=client"
  refute_output --partial "file in check output not found"
}

@test "HK_MISE_PER_STEP applies workspace-relative check_diff" {
  setup_mise_per_step_repo

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["diff"] {
        glob = "*.go"
        check_diff = "printf -- '--- main.go\n+++ main.go\n@@ -1 +1 @@\n-bad\n+good\n'; exit 1"
        fix = "printf fallback"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk fix --all --step diff -v
  assert_success
  assert_output --partial "diff applied successfully"
  run cat packages/client/main.go
  assert_success
  assert_output "good"
}

@test "HK_MISE_PER_STEP applies repo-relative check_diff from workspace cwd" {
  setup_mise_per_step_repo

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      ["diff"] {
        glob = "packages/client/*.go"
        check_diff = "printf -- '--- packages/client/main.go\n+++ packages/client/main.go\n@@ -1 +1 @@\n-bad\n+good\n'; exit 1"
        fix = "printf fallback"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk fix --all --step diff -v
  assert_success
  assert_output --partial "diff applied successfully"
  run cat packages/client/main.go
  assert_success
  assert_output "good"
}

@test "HK_MISE_PER_STEP preserves explicit workspace_indicator cwd" {
  setup_mise_per_step_repo
  touch packages/client/go.mod
  git add .

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["explicit"] {
        glob = "packages/client/*.go"
        workspace_indicator = "go.mod"
        check = "printf 'pwd=%s files=%s workspace=%s env=%s' \$(pwd) '{{files}}' '{{workspace}}' \${HK_TEST_WORKSPACE-unset}"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk check --all --step explicit -v
  assert_success
  assert_output --partial "files=packages/client/main.go workspace=packages/client env=unset"
  refute_output --partial "files=main.go workspace=packages/client env=client"
}

@test "HK_MISE_PER_STEP respects configured shell when wrapping" {
  setup_mise_per_step_repo

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["bash-shell"] {
        glob = "packages/client/*.go"
        shell = "bash -e -c"
        check = "[[ \"\$HK_TEST_WORKSPACE\" == client ]] && nested-tool {{files}}"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk check --all --step bash-shell -v
  assert_success
  assert_output --partial "mise x -- bash -e -c"
  assert_output --partial "nested-tool:"
  assert_output --partial "args=main.go env=client"
}

@test "HK_MISE_PER_STEP keeps root files outside nested mise roots" {
  setup_mise_per_step_repo
  printf 'root\n' > root.go
  git add .

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["mixed"] {
        glob = "**/*.go"
        check = "if command -v nested-tool >/dev/null 2>&1; then nested-tool {{files}}; else printf 'root-files=%s' '{{files}}'; fi"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk check --all --step mixed -v
  assert_success
  assert_output --partial "nested-tool:"
  assert_output --partial "args=main.go env=client"
  assert_output --partial "root-files=root.go"
}

@test "HK_MISE_PER_STEP does not create empty jobs for root mise config" {
  setup_mise_per_step_repo
  printf 'root\n' > root.go
  git add .

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["root-only"] {
        glob = "root.go"
        check = "printf 'files=%s' '{{files}}'"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk check --all --step root-only -v
  assert_success
  assert_output --partial "files=root.go"
  refute_output --partial "0 files"
}

@test "HK_MISE_PER_STEP does not split or wrap pure hk util steps" {
  setup_mise_per_step_repo

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["hk-util"] {
        glob = "**/*.go"
        check = "hk util check-byte-order-marker {{files}}"
      }
    }
  }
}
EOF

  run env HK_MISE_PER_STEP=1 hk check --all --step hk-util -v
  assert_success
  assert_output --partial "hk util check-byte-order-marker packages/client/main.go packages/server/main.go"
  refute_output --partial "mise x --"
  refute_output --partial "hk util check-byte-order-marker main.go"
}

@test "HK_MISE_PER_STEP resolves tools from explicit dir" {
  setup_mise_per_step_repo

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["explicit-dir"] {
        glob = "*.go"
        dir = "packages/client"
        check = "nested-tool {{files}}"
      }
    }
  }
}
EOF

  run hk check --all --step explicit-dir -v
  assert_failure
  assert_output --partial "nested-tool: command not found"

  run env HK_MISE_PER_STEP=1 hk check --all --step explicit-dir -v
  assert_success
  assert_output --partial "nested-tool:"
  assert_output --partial "args=main.go env=client"
}
