#!/usr/bin/env bats

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

@test "stage globs stage all matching files including untracked" {
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ["prettier"] {
        glob = "src/changed.ts"
        stage = "**/*.ts"
        fix = "printf 'fixed\n' >> src/changed.ts"
      }
    }
  }
}
PKL
  git add hk.pkl
  git -c commit.gpgsign=false commit -m "init hk"
  hk install

  mkdir -p src
  printf 'one\n' > src/changed.ts
  git add src/changed.ts
  git -c commit.gpgsign=false commit -m "add changed"

  # Create an untracked file that matches the glob
  printf 'two\n' > src/unrelated.ts
  printf 'one\nmore\n' > src/changed.ts

  run hk fix -v
  assert_success

  # Both files match **/*.ts so both should be staged
  run git status --porcelain
  assert_success
  assert_line --regexp '^[MA]  src/changed\.ts$'
  assert_line --regexp '^A  src/unrelated\.ts$'
}

@test "stage globs do not stage files outside the glob pattern" {
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ["prettier"] {
        glob = "src/changed.ts"
        stage = "**/*.ts"
        fix = """
printf 'fixed\n' >> src/changed.ts
printf 'created\n' > src/created.md
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
  printf 'one\n' > src/changed.ts
  git add src/changed.ts
  git -c commit.gpgsign=false commit -m "add changed"

  # Create files that DON'T match the glob
  printf 'preexisting\n' > src/preexisting.md

  # Now modify the job file
  printf 'one\nmore\n' > src/changed.ts

  run hk fix -v
  assert_success

  # changed.ts should be staged (matches glob), but .md files should not
  run git status --porcelain
  assert_success
  assert_line --regexp '^[MA]  src/changed\.ts$'
  refute_line --regexp '^[MA]  src/preexisting\.md$'
  refute_line --regexp '^[MA]  src/created\.md$'
  # .md files should remain untracked
  assert_line --regexp '^\?\? src/created\.md$'
  assert_line --regexp '^\?\? src/preexisting\.md$'
}

@test "stage globs DO stage newly created files by the step" {
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ["create-file"] {
        glob = "src/changed.ts"
        stage = "**/*.ts"
        fix = """
printf 'fixed\n' >> src/changed.ts
printf 'created by step\n' > src/created_by_step.ts
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
  printf 'one\n' > src/changed.ts
  git add src/changed.ts
  git -c commit.gpgsign=false commit -m "add changed"

  printf 'one\nmore\n' > src/changed.ts

  run hk fix -v
  assert_success

  # Both changed.ts AND created_by_step.ts should be staged
  run git status --porcelain
  assert_success
  assert_line --regexp '^[MA]  src/changed\.ts$'
  assert_line --regexp '^A  src/created_by_step\.ts$'
}

@test "stage globs stage pre-existing unstaged modified files that match" {
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ["prettier"] {
        glob = "src/changed.ts"
        stage = "**/*.ts"
        fix = "printf 'fixed\n' >> src/changed.ts"
      }
    }
  }
}
PKL
  git add hk.pkl
  git -c commit.gpgsign=false commit -m "init hk"
  hk install

  mkdir -p src
  printf 'one\n' > src/changed.ts
  printf 'two\n' > src/other.ts
  git add src/changed.ts src/other.ts
  git -c commit.gpgsign=false commit -m "add files"

  # Modify both files
  printf 'one\nmore\n' > src/changed.ts
  printf 'two\nmore\n' > src/other.ts

  run hk fix -v
  assert_success

  # Both files match **/*.ts so both should be staged
  run git status --porcelain
  assert_success
  assert_line --regexp '^M  src/changed\.ts$'
  assert_line --regexp '^M  src/other\.ts$'
}
