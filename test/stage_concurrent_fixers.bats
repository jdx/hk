#!/usr/bin/env bats

# Staging after a step's fixes must never read a file that another step is
# still writing. Otherwise libgit2 fails with "file changed before we could
# read it", or the index receives a partially written file.

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

@test "staging waits for a concurrent fixer to finish writing" {
  export HK_JOBS=4
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "none"
    steps {
      // Waits until "slow" is mid-write, then finishes and stages with a
      // glob that also matches data.json.
      ["fast"] {
        glob = "*.txt"
        stage = "*"
        fix = """
          for _ in \$(seq 100); do grep -q PARTIAL data.json && break; sleep 0.05; done
          grep -q PARTIAL data.json || { echo 'slow never started' >&2; exit 1; }
          echo fixed > {{files}}
          """
      }
      // Holds data.json in a partially written state while "fast" stages.
      ["slow"] {
        glob = "*.json"
        fix = """
          printf 'PARTIAL' > {{files}}
          sleep 2
          if git show :{{files}} | grep -q PARTIAL; then echo staged-partial > ../race; fi
          echo '{}' > {{files}}
          """
      }
    }
  }
}
PKL
  git add hk.pkl
  git commit -qm "init hk"

  printf 'broken\n' > a.txt
  printf '{ }\n' > data.json
  git add a.txt data.json

  run hk run pre-commit
  assert_success
  assert_file_not_exists ../race

  run git show :data.json
  assert_output '{}'
  run git status --porcelain
  assert_output "$(printf 'A  a.txt\nA  data.json')"
}

@test "git status during staging waits for a concurrent fixer" {
  export HK_JOBS=4
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "none"
    steps {
      // Stages only its own file, but staging scans the whole worktree status.
      ["fast"] {
        glob = "*.txt"
        fix = """
          for _ in \$(seq 100); do test -e ../slow-started && break; sleep 0.05; done
          test -e ../slow-started || { echo 'slow never started' >&2; exit 1; }
          echo fixed > {{files}}
          """
      }
      // Keeps rewriting the JSON files at their staged size, so git must hash
      // their contents to tell whether they changed. It stops once "fast" has
      // staged (only possible without locking) or after a few seconds (with
      // locking, "fast" cannot stage until this step finishes writing).
      ["slow"] {
        glob = "*.json"
        fix = """
          touch ../slow-started
          end=\$((\$(date +%s) + 3))
          rewrite() {
            while [ "\$(git show :a.txt)" != fixed ] && [ \$(date +%s) -lt \$end ]; do
              for f in {{files}}; do yes y | head -c 200000 > "\$f"; done
            done
          }
          rewrite & rewrite
          wait
          for f in {{files}}; do echo '{}' > "\$f"; done
          """
      }
    }
  }
}
PKL
  git add hk.pkl
  git commit -qm "init hk"

  printf 'broken\n' > a.txt
  for i in $(seq 1 40); do
    yes x | head -c 200000 > "data$i.json"
  done
  git add .

  run hk run pre-commit
  assert_success

  run git show :a.txt
  assert_output 'fixed'
  for i in $(seq 1 40); do
    run git show ":data$i.json"
    assert_output '{}'
  done
}

@test "overlapping fixers leave every file fixed and staged" {
  export HK_JOBS=8
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "none"
    steps {
      ["trailing-whitespace"] = Builtins.trailing_whitespace
      ["newlines"] = Builtins.newlines
      ["jq"] = Builtins.jq
    }
  }
}
PKL
  git add hk.pkl
  git commit -qm "init hk"

  for trial in 1 2 3 4 5; do
    for i in $(seq 1 60); do
      printf '{"b": %d, "a": "x"}   ' "$i" > "file$i.json"
      printf 'line %d   \nlast' "$i" > "file$i.txt"
    done
    git add .

    run hk run pre-commit
    assert_success

    # Everything is fixed and staged; nothing is left in the worktree.
    run git status --porcelain --untracked-files=no
    refute_output --regexp '^.[MD]'
    for i in $(seq 1 60); do
      run git show ":file$i.json"
      assert_output "$(printf '{\n  "a": "x",\n  "b": %d\n}' "$i")"
      run git show ":file$i.txt"
      assert_output "$(printf 'line %d\nlast' "$i")"
    done

    git commit -qm "trial $trial" --no-verify --allow-empty
  done
}

@test "a failed git add fails every step whose fixes it was staging" {
  export HK_JOBS=4
  # Steps that stage at the same time share one `git add`. Whether these two
  # share one depends on timing, so this checks the outcome either way: each
  # step reports the failure of the `git add` that held its files.
  real_git="$(command -v git)"
  mkdir -p "$TEST_TEMP_DIR/bin"
  cat <<SH > "$TEST_TEMP_DIR/bin/git"
#!/bin/sh
if [ "\$1" = add ]; then
  echo "simulated git add failure" >&2
  exit 1
fi
exec "$real_git" "\$@"
SH
  chmod +x "$TEST_TEMP_DIR/bin/git"
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
fail_fast = false
hooks {
  ["pre-commit"] {
    fix = true
    stash = "none"
    steps {
      ["fix-a"] {
        glob = "a.txt"
        fix = "echo fixed > {{files}}"
      }
      ["fix-b"] {
        glob = "b.txt"
        fix = "echo fixed > {{files}}"
      }
    }
  }
}
PKL
  git add hk.pkl
  git commit -qm "init hk"

  echo broken > a.txt
  echo broken > b.txt
  git add a.txt b.txt

  PATH="$TEST_TEMP_DIR/bin:$PATH" run hk run pre-commit
  assert_failure
  assert_output --partial "simulated git add failure"
  assert_output --partial "✗ fix-a – ERROR"
  assert_output --partial "✗ fix-b – ERROR"
  assert_output --partial "hook finished with error: exited with code 1"
  run git status --porcelain
  assert_output "$(printf 'AM a.txt\nAM b.txt')"
}
