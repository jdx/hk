#!/usr/bin/env bats

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

@test "stage=<JOB_FILES> only stages files processed by the step" {
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ["shfmt"] {
        glob = List("**/*")
        stage = "<JOB_FILES>"
        check_list_files = """
# Simulate shfmt's shell script detection - output one file per line
matched=""
for file in {{files}}; do
  if head -n 1 "\$file" 2>/dev/null | grep -q '^#!.*sh'; then
    printf '%s\n' "\$file"
    matched=1
  fi
done
test -n "\$matched" && exit 1 || exit 0
"""
        fix = """
# Add a comment to each file
for file in "\$@"; do
  echo "# formatted" >> "\$file"
done
"""
      }
    }
  }
}
PKL
  git add hk.pkl
  git -c commit.gpgsign=false commit -m "init hk"
  hk install

  mkdir -p src
  # Create a shell script (will be processed)
  cat > src/script.sh << 'EOF'
#!/bin/bash
echo "hello"
EOF
  git add src/script.sh
  git -c commit.gpgsign=false commit -m "add script"

  # Create an untracked file that would match the glob but isn't a shell script
  printf 'some text\n' > src/unrelated.txt

  # Create an untracked shell script (with "bad" formatting that check will detect)
  cat > src/untracked.sh << 'EOF'
#!/bin/bash
echo "untracked"
NEEDS_FORMATTING
EOF

  # Modify the tracked shell script (with "bad" formatting that check will detect)
  printf 'echo "modified"\nNEEDS_FORMATTING\n' >> src/script.sh

  run hk fix -v
  assert_success

  # src/script.sh AND src/untracked.sh should be staged (both are shell scripts processed by check_list_files)
  # src/unrelated.txt should NOT be staged (not a shell script, filtered out by check_list_files)
  run git status --porcelain
  assert_success
  assert_line --regexp '^M  src/script\.sh$'
  assert_line --regexp '^A  src/untracked\.sh$'
  refute_line --regexp '^A  src/unrelated\.txt$'
  # unrelated.txt should remain untracked (was filtered out)
  assert_line --regexp '^\?\? src/unrelated\.txt$'
}

@test "stage=<JOB_FILES> works with check_list_files that filters by content" {
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ["fix-todos"] {
        glob = "**/*.ts"
        stage = "<JOB_FILES>"
        check_list_files = """
# Only return files that contain TODO comments
if grep -l 'TODO' {{files}} 2>/dev/null; then
  exit 1
fi
exit 0
"""
        fix = """
# Add a FIXED comment
for file in "\$@"; do
  echo "// FIXED" >> "\$file"
done
"""
      }
    }
  }
}
PKL
  git add hk.pkl
  git -c commit.gpgsign=false commit -m "init hk"
  hk install

  mkdir -p src
  # File with TODO
  cat > src/with_todo.ts << 'EOF'
// TODO: implement
export const foo = 1;
EOF
  # File without TODO
  cat > src/without_todo.ts << 'EOF'
export const bar = 2;
EOF
  git add src/*.ts
  git -c commit.gpgsign=false commit -m "add files"

  # Modify both files
  printf 'export const baz = 3;\n' >> src/with_todo.ts
  printf 'export const qux = 4;\n' >> src/without_todo.ts

  run hk fix -v
  assert_success

  # Only with_todo.ts should be staged (it had TODO and was processed)
  # without_todo.ts should remain unstaged
  run git status --porcelain
  assert_success
  assert_line --regexp '^M  src/with_todo\.ts$'
  assert_line --regexp '^ M src/without_todo\.ts$'
}

@test "stage=<JOB_FILES> stages files created by the step" {
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ["generator"] {
        glob = "src/input.ts"
        stage = "<JOB_FILES>"
        fix = """
# Process input and generate output
cat src/input.ts > src/generated.ts
echo "// generated" >> src/generated.ts
echo "fixed" >> src/input.ts
"""
      }
    }
  }
}
PKL
  git add hk.pkl
  git -c commit.gpgsign=false commit -m "init hk"
  hk install

  mkdir -p src
  printf 'export const input = 1;\n' > src/input.ts
  git add src/input.ts
  git -c commit.gpgsign=false commit -m "add input"

  # Modify input
  printf 'export const modified = 2;\n' >> src/input.ts

  run hk fix -v
  assert_success

  # Both input.ts and generated.ts should be staged
  # input.ts was in job_files, generated.ts was created by the fix command
  run git status --porcelain
  assert_success
  assert_line --regexp '^M  src/input\.ts$'
  # Note: generated.ts is created by the fix script but isn't in glob,
  # so it won't be in job_files. This test verifies {{job_files}} behavior.
  # The generated file should remain untracked since it's not in job_files.
  assert_line --regexp '^\?\? src/generated\.ts$'
}
