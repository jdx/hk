#!/usr/bin/env mise run test:bats

setup() {
  load 'test_helper/common_setup'
  _common_setup
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

// Test configuration with custom display_skip_reasons settings
display_skip_reasons = List()  // Empty list means hide all skip messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-profile"] {
                profiles = List("nonexistent")
                check = "echo 'test profile'"
            }
            ["test-no-command"] {
                // Step with no check or fix command
            }
            ["test-normal"] {
                check = "echo 'normal test'"
                glob = "*.nonexistent"
            }
        }
    }
}
EOF
}

teardown() {
  _common_teardown
}

@test "skip_reasons: ProfileNotEnabled messages are hidden when configured" {
  # Run with default profile (nonexistent profile won't be enabled)
  run hk check --all
  assert_success

  # ProfileNotEnabled is set to false, so this message should NOT appear
  refute_output --partial "skipped: disabled by profile"

  # The step should still be skipped but silently
  refute_output --partial "test-profile"
}

@test "skip_reasons: Default configuration shows ProfileNotEnabled" {
  # Create a config with default skipReasons (or explicitly set profileNotEnabled)
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List("profile-not-enabled")  // Show profile not enabled messages (default)

hooks = new {
    ["check"] {
        steps = new {
            ["test-profile"] {
                profiles = List("nonexistent")
                check = "echo 'test profile'"
            }
        }
    }
}
EOF

  run hk check --all
  assert_success

  # ProfileNotEnabled is set to true, so this message SHOULD appear
  assert_output --partial "skipped: profile not enabled (nonexistent)"
}

@test "skip_reasons: NoCommandForRunType messages can be configured" {
  # First test with NoCommandForRunType hidden
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List()  // Empty list hides all messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-no-command"] {
                // Step with no check or fix command
            }
        }
    }
}
EOF

  run hk check --all
  assert_success

  # NoCommandForRunType is set to false, so this message should NOT appear
  refute_output --partial "skipped: no command for run type"

  # Now test with NoCommandForRunType shown
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List("no-command-for-run-type")  // Show no command messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-no-command"] {
                // Step with no check or fix command
            }
        }
    }
}
EOF

  run hk check --all
  assert_success

  # NoCommandForRunType is set to true, so this message SHOULD appear
  assert_output --partial "skipped: no command for run type"
}

@test "skip_reasons: DisabledByEnv skip messages can be configured" {
  # Test with DisabledByEnv messages hidden
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List()  // Empty list hides all messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-step"] {
                check = "echo 'test'"
            }
        }
    }
}
EOF

  HK_SKIP_STEPS="test-step" run hk check --all
  assert_success

  # DisabledByEnv is not configured, so skip message should NOT appear
  refute_output --partial "skipped: disabled via HK_SKIP_STEPS"

  # Now test with DisabledByEnv messages shown
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List("disabled-by-env")  // Show environment skip messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-step"] {
                check = "echo 'test'"
            }
        }
    }
}
EOF

  HK_SKIP_STEPS="test-step" run hk check --all
  assert_success

  # DisabledByEnv is configured, so skip message SHOULD appear
  assert_output --partial "skipped: disabled via HK_SKIP_STEPS"
}

@test "skip_reasons: NoFilesToProcess messages can be configured" {
  # First test with NoFilesToProcess hidden
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List()  // Empty list hides all messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-glob"] {
                check = "echo 'test'"
                glob = "*.nonexistent"  // Will match no files
            }
        }
    }
}
EOF

  run hk check --all
  assert_success

  # NoFilesToProcess is not in the list, so message should NOT appear
  refute_output --partial "skipped: no files to process"

  # Now test with NoFilesToProcess shown
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List("no-files-to-process")  // Show no files messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-glob"] {
                check = "echo 'test'"
                glob = "*.nonexistent"  // Will match no files
            }
        }
    }
}
EOF

  run hk check --all
  assert_success

  # NoFilesToProcess is in the list, so message SHOULD appear
  assert_output --partial "skipped: no files to process"
}

@test "skip_reasons: DisabledByCli skip messages can be configured" {
  # Test with DisabledByCli messages hidden
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List()  // Empty list hides all messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-step"] {
                check = "echo 'test'"
            }
        }
    }
}
EOF

  run hk check --all --skip-step test-step
  assert_success

  # DisabledByCli is not configured, so skip message should NOT appear
  refute_output --partial "skipped: disabled via --skip-step"

  # Now test with DisabledByCli messages shown
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

display_skip_reasons = List("disabled-by-cli")  // Show CLI skip messages

hooks = new {
    ["check"] {
        steps = new {
            ["test-step"] {
                check = "echo 'test'"
            }
        }
    }
}
EOF

  run hk check --all --skip-step test-step
  assert_success

  # DisabledByCli is configured, so skip message SHOULD appear
  assert_output --partial "skipped: disabled via --skip-step"
}

