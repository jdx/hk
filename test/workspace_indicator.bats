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
    check = "echo \"ws={{workspace}}; files={{files}}; wfiles={{workspace_files}}\""
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
  assert_output --partial "echo \"ws=.; files=main.go; wfiles=main.go\""
  # Workspace a
  assert_output --partial "echo \"ws=a; files=a/main.go; wfiles=main.go\""
  # Workspace b
  assert_output --partial "echo \"ws=b; files=b/main.go; wfiles=main.go\""

  # Should NOT see a/main.go or b/main.go in the root workspace's echo
  # (i.e., no echo with multiple files)
  refute_output --partial "files=a/main.go b/main.go main.go"
  refute_output --partial "files=a/main.go main.go"
  refute_output --partial "files=b/main.go main.go"
}

@test "workspace_indicator respects batch=true" {
  # With HK_JOBS=2 and 8 files in one workspace, chunk_size = total_files/jobs = 8/2 = 4,
  # so the workspace's 8 files should be split into 2 batched jobs of 4 files each.
  rm -rf a b go.mod main.go hk.pkl
  mkdir -p ws1
  touch ws1/package.json
  for i in $(seq 1 8); do
    echo "f$i" > ws1/f$i.js
  done

  cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["eslint"] {
        glob = "**/*.js"
        workspace_indicator = "package.json"
        batch = true
        check = "echo \"ws={{workspace}}; files={{files}}\""
      }
    }
  }
}
EOF

  git add .

  run hk check --all -v
  assert_success

  # Count actual command executions (the "$ echo" debug lines), not output lines
  job_count=$(echo "$output" | grep -c '^\s*DEBUG \$ echo "ws=')
  [ "$job_count" -eq 2 ]

  # No single job should contain all 8 files — they must be split across batches
  refute_output --partial "ws1/f1.js ws1/f2.js ws1/f3.js ws1/f4.js ws1/f5.js ws1/f6.js ws1/f7.js ws1/f8.js"
}
