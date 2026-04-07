#!/usr/bin/env bats

setup() {
  load 'test_helper/common_setup'
  _common_setup
}

teardown() {
  _common_teardown
}

@test "required: step is skipped when env is missing" {
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List("missing-required-env")

hooks = new {
    ["check"] {
        steps = new {
            ["test-required"] {
                required = List("MANDATORY_VAR")
                check = "echo 'Success'"
            }
        }
    }
}
EOF

  run hk check --all
  assert_success
  assert_output --partial "skipped: missing required environment variable(s): MANDATORY_VAR"
}

@test "required: step runs when env is provided" {
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks = new {
    ["check"] {
        steps = new {
            ["test-required"] {
                required = List("MANDATORY_VAR")
                check = "echo 'Success'"
            }
        }
    }
}
EOF

  MANDATORY_VAR=1 run hk check --all
  assert_success
  assert_output --partial "test-required"
  refute_output --partial "skipped"
}

@test "required: step runs when env is defined in step env block" {
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks = new {
    ["check"] {
        steps = new {
            ["test-required"] {
                required = List("MANDATORY_VAR")
                check = "echo 'Success'"
                env {
                    ["MANDATORY_VAR"] = "1"
                }
            }
        }
    }
}
EOF

  run hk check --all
  assert_success
  assert_output --partial "test-required"
  refute_output --partial "skipped"
}
