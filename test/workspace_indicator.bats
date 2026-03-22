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
  # Create two workspaces with enough files to trigger batching (HK_JOBS=2)
  rm -rf a b go.mod main.go hk.pkl
  mkdir -p ws1 ws2
  touch ws1/package.json ws2/package.json
  for i in 1 2 3 4; do
    echo "f$i" > ws1/f$i.js
    echo "f$i" > ws2/f$i.js
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

  # With batch=true and HK_JOBS=2, each workspace's 4 files should be split
  # into 2 batched jobs (2 files each) rather than 1 job with all 4 files.
  # Count how many times the check command ran — should be more than 2
  # (which would be 1 job per workspace without batching).
  job_count=$(echo "$output" | grep -c 'echo "ws=')
  [ "$job_count" -gt 2 ]

  # Files from different workspaces should never be mixed in the same job
  refute_output --partial "ws1/f1.js ws2/"
  refute_output --partial "ws2/f1.js ws1/"
}
