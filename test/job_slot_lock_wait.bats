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
      // the two slots when the hook starts, and the one that runs first waits
      // for "other" to run before it finishes.
      ["first"] {
        glob = "shared.txt"
        fix = "for _ in \$(seq 100); do test -e ../other-ran && break; sleep 0.1; done; test -e ../other-ran && echo ran >> ../writers || echo waited >> ../writers"
      }
      ["second"] {
        glob = "shared.txt"
        fix = "for _ in \$(seq 100); do test -e ../other-ran && break; sleep 0.1; done; test -e ../other-ran && echo ran >> ../writers || echo waited >> ../writers"
      }
      // Starts without a slot. It can run only if the writer waiting for
      // shared.txt gives up its slot.
      ["other"] {
        glob = "other.txt"
        fix = "touch ../other-ran"
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
  # Holding its slot, the waiting writer would leave "other" queued until the
  # first writer gave up after 10 seconds.
  run cat ../writers
  assert_output "$(printf 'ran\nran')"
}
