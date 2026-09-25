#!/usr/bin/env bats

# A job waiting for another step to release its files must not hold one of the
# `--jobs` slots, or steps with nothing to wait for queue behind it.

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

@test "a job waiting for file locks leaves its slot to a job that can run" {
  export HK_JOBS=2
  cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    fix = true
    steps {
      // Both write shared.txt, so one waits for the other. Each takes one of
      // the two slots when the hook starts.
      ["first"] {
        glob = "shared.txt"
        fix = "sleep 2; echo first >> ../writers-done"
      }
      ["second"] {
        glob = "shared.txt"
        fix = "sleep 2; echo second >> ../writers-done"
      }
      // Starts without a slot. It should get the waiting writer's slot and
      // run while the other writer is still going.
      ["other"] {
        glob = "other.txt"
        fix = "cat ../writers-done 2>/dev/null | wc -l | tr -d ' ' > ../writers-done-before-other"
      }
    }
  }
}
PKL
  touch shared.txt other.txt
  git add -A
  git commit -qm init

  run hk fix --all
  assert_success
  run cat ../writers-done-before-other
  assert_output 0
}
