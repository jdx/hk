#!/usr/bin/env bats

setup() {
  load 'test_helper/common_setup'
  _common_setup
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

// Test configuration with custom skipReasons settings
skipReasons = List()  // Empty list means hide all skip messages

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

skipReasons = List("profileNotEnabled")  // Show profile not enabled messages (default)

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
  assert_output --partial "skipped: disabled by profile"
}

@test "skip_reasons: NoCommandForRunType messages can be configured" {
  # First test with NoCommandForRunType hidden
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

skipReasons = List()  // Empty list hides all messages

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

skipReasons = List("noCommandForRunType")  // Show no command messages

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

@test "skip_reasons: Env skip messages can be configured" {
  # Test with Env messages hidden
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

skipReasons = List()  // Empty list hides all messages

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
  
  # Env is set to false, so skip message should NOT appear
  refute_output --partial "skipped: disabled via HK_SKIP_STEPS"
  
  # Now test with Env messages shown
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

skipReasons = List("env")  // Show environment skip messages

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
  
  # Env is set to true, so skip message SHOULD appear
  assert_output --partial "skipped: disabled via HK_SKIP_STEPS"
}

@test "skip_reasons: Cli skip messages can be configured" {
  # Test with Cli messages hidden
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

skipReasons = List()  // Empty list hides all messages

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
  
  # Cli is set to false, so skip message should NOT appear
  refute_output --partial "skipped: disabled via --skip-step"
  
  # Now test with Cli messages shown
  cat >hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

skipReasons = List("cli")  // Show CLI skip messages

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
  
  # Cli is set to true, so skip message SHOULD appear
  assert_output --partial "skipped: disabled via --skip-step"
}
