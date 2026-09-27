#!/usr/bin/env bats

# Before staging, hk reads the index to find racily clean entries, which
# `git add` may re-hash while another step is rewriting them. hk reads index
# versions 2 to 4 itself and leaves other formats, such as a split index, to
# libgit2. Staging must wait for the other step in every format.

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

# Runs a hook in which "fast" stages its fix while "slow" is rewriting a file
# whose index entry is racily clean. `$@` sets up the index format.
stage_while_racily_clean_file_is_rewritten() {
  export HK_JOBS=4
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    fix = true
    stash = "none"
    steps {
      // Stages only its own file once "slow" is mid-write.
      ["fast"] {
        glob = "*.txt"
        fix = """
          for _ in \$(seq 100); do test -e ../slow-started && break; sleep 0.05; done
          test -e ../slow-started || { echo 'slow never started' >&2; exit 1; }
          echo fixed > {{files}}
          """
      }
      // Rewrites data.json, and records whether "fast" staged meanwhile.
      ["slow"] {
        glob = "*.json"
        fix = """
          printf 'PARTIAL' > {{files}}
          touch ../slow-started
          sleep 2
          if [ "\$(git show :a.txt)" = fixed ]; then echo staged-mid-write > ../race; fi
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
  # An mtime in the future keeps the entry racily clean however often the
  # index is rewritten.
  touch -t "$(($(date +%Y) + 1))01010000" data.json
  git add a.txt data.json
  "$@"

  HK_LOG=debug run hk run pre-commit
  assert_success
  assert_file_not_exists ../race
  hook_output=$output

  run git show :a.txt
  assert_output 'fixed'
  run git show :data.json
  assert_output '{}'
}

@test "staging waits for a racily clean file with index version 2" {
  stage_while_racily_clean_file_is_rewritten git update-index --index-version 2
  output=$hook_output
  refute_output --partial "with libgit2"
  # The index version is the header's fourth byte. macOS pads od's output.
  run sh -c "od -An -tu1 -j7 -N1 .git/index | tr -d '[:space:]'"
  assert_output '2'
}

@test "staging waits for a racily clean file with index version 4" {
  stage_while_racily_clean_file_is_rewritten git update-index --index-version 4
  output=$hook_output
  refute_output --partial "with libgit2"
  # The index version is the header's fourth byte. macOS pads od's output.
  run sh -c "od -An -tu1 -j7 -N1 .git/index | tr -d '[:space:]'"
  assert_output '4'
}

# git writes the index unsplit when hk refreshes it at the start of the hook,
# and split again from the first `git add`, so the step that stages second
# reads a split index. hk leaves it to libgit2, which declines it, so hk locks
# every file in the hook, as it did before it read the index itself.
@test "staging waits for a racily clean file with a split index" {
  git config core.splitIndex true
  stage_while_racily_clean_file_is_rewritten git update-index --split-index
  output=$hook_output
  assert_output --partial "with libgit2"
  assert_output --partial "locking all files"
  run ls .git
  assert_output --partial sharedindex.
}
