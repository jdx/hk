#!/usr/bin/env bats

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

@test "prettier stage globs do not over-stage unrelated files" {
  cat <<'PKL' > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps = new Mapping<String, Step> {
      ["prettier"] {
        glob = "src/changed.ts"
        stage = List("**/*.ts", "src/explicit.ts")
        fix = "printf 'fixed\n' >> src/changed.ts"
      }
    }
  }
}
PKL
  git add hk.pkl
  git -c commit.gpgsign=false commit -m "init hk"

  mkdir -p src
  printf 'one\n' > src/changed.ts
  printf 'two\n' > src/unrelated.ts
  git add src/changed.ts
  git -c commit.gpgsign=false commit -m "add changed"

  printf 'one\nmore\n' > src/changed.ts
  printf 'two\nmore\n' > src/unrelated.ts

  run hk fix -v
  assert_success

  # Only the job file should be staged; unrelated.ts remains unstaged
  run git status --porcelain -- src/changed.ts src/unrelated.ts
  assert_success
  # changed.ts should be staged (M or A); unrelated.ts unstaged (worktree change only)
  assert_line --regexp '^[MA]  src/changed\.ts$'
  refute_line --regexp '^[MA]  src/unrelated\.ts$'
}
