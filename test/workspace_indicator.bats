#!/usr/bin/env bats

setup() {
  load 'test_helper/common_setup'
  _common_setup
  _workspace_indicator_setup
}

teardown() {
  _common_teardown
}

_workspace_indicator_setup() {
  # Use current directory for all setup
  mkdir -p a b

  touch go.mod main.go
  touch a/go.mod a/main.go
  touch b/go.mod b/main.go

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

local linters = new Mapping<String, Step> {
  ["golangci-lint"] {
    glob = "*.go"
    workspace_indicator = "go.mod"
    check = "echo {{ files }}"
  }
}

hooks {
  ["check"] {
    steps = linters
  }
}
EOF

  git init -q
  git add .
}

@test "each workspace only processes its own files" {
  run hk check -v

  # Should see three jobs, one for each workspace, each with only its own file
  # Root workspace
  assert_output --partial "echo main.go"
  # Workspace a
  assert_output --partial "echo a/main.go"
  # Workspace b
  assert_output --partial "echo b/main.go"

  # Should NOT see a/main.go or b/main.go in the root workspace's echo
  # (i.e., no echo with multiple files)
  refute_output --partial "echo a/main.go b/main.go main.go"
  refute_output --partial "echo a/main.go main.go"
  refute_output --partial "echo b/main.go main.go"
} 
