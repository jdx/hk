#!/usr/bin/env bats

# Test Pkl configuration error messages

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "missing amends declaration shows helpful error" {
    # Create a Pkl config without amends declaration
    cat > hk.pkl << 'EOF'
hooks {
  ["check"] = new Hook {
    steps {
      ["test"] = new Step {
        run = "echo test"
      }
    }
  }
}
EOF

    # Run hk and expect it to fail with helpful error message
    run hk check
    assert_failure

    # Check that the error message contains helpful information
    assert_output --partial
    assert_output --partial "Missing 'amends' declaration"
    assert_output --partial "Your hk.pkl file should start with one of:"
    assert_output --partial "amends \"pkl/Config.pkl\""
    assert_output --partial "amends \"package://github.com/jdx/hk"
}

@test "invalid module URI shows helpful error" {
    # Create a Pkl config with invalid module URI
    cat > hk.pkl << 'EOF'
amends "package://hk.sh/Config.pkl"

hooks {
  ["check"] = new Hook {
    steps {
      ["test"] = new Step {
        run = "echo test"
      }
    }
  }
}
EOF

    # Run hk and expect it to fail with helpful error message
    run hk check
    assert_failure

    # Check that the error message contains helpful information
    assert_output --partial "Invalid module URI"
    assert_output --partial "Make sure your 'amends' declaration uses a valid path or package URL"
}

@test "pkl file with syntax errors shows original error" {
    # Create a Pkl config with syntax errors
    cat > hk.pkl << 'EOF'
amends "../pkl/Config.pkl"

hooks = { this is invalid syntax
EOF

    # Run hk and expect it to fail
    run hk check
    assert_failure

    # Should show the Pkl error (not our custom messages)
    assert_output --partial "Failed to evaluate Pkl config"
}
