#!/usr/bin/env bash
# Test that reproduces the shfmt untracked file staging bug
# Bug: when shfmt has glob="**/*" and stage="**/*", it stages untracked files
# that happen to be shell scripts during a commit
# Fixed by using stage="<JOB_FILES>" instead

set -euo pipefail

TEST_DIR=$(mktemp -d)
trap 'rm -rf "$TEST_DIR"' EXIT

cd "$TEST_DIR"

# Initialize a git repo
git init
git config user.email "test@test.com"
git config user.name "Test User"

# Create a minimal hk.pkl config with shfmt that has the bug
cat > hk.pkl <<'EOF'
amends "package://github.com/jdx/hk/releases/download/v1.18.2/hk@1.18.2#/Config.pkl"

hooks = new {
  ["pre-commit"] {
    fix = true
    stash = "git"
    steps {
      ["shfmt"] {
        glob = List("**/*")
        stage = List("**/*")
        check_list_files = "find . -name '*.sh' -type f"
        fix = "shfmt -w {{files}}"
      }
    }
  }
}
EOF

# Install hk hooks
hk install

# Create and commit the hk.pkl file
git add hk.pkl
git commit -m "initial commit"

# Create an untracked shell script
cat > test-untracked.sh <<'EOF'
#!/bin/sh

  echo "unformatted"
EOF

# Modify hk.pkl to trigger a commit
echo "// test comment" >> hk.pkl

# Stage hk.pkl and commit
git add hk.pkl

# Before commit: verify test-untracked.sh is not staged
if git diff --cached --name-only | grep -q "test-untracked.sh"; then
  echo "FAIL: test-untracked.sh is already staged before commit"
  exit 1
fi

# Make the commit
git commit -m "test commit"

# After commit: check if test-untracked.sh was staged and committed
if git log -1 --name-only | grep -q "test-untracked.sh"; then
  echo "FAIL: test-untracked.sh was incorrectly staged and committed"
  echo "Files in commit:"
  git log -1 --name-only
  exit 1
fi

echo "PASS: test-untracked.sh was not staged during commit"
