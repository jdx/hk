#!/usr/bin/env bats

# Test Pkl configuration error messages

setup() {
    export HK="${HK:-$BATS_TEST_DIRNAME/../target/debug/hk}"
    export TEST_DIR="$(mktemp -d)"
    cd "$TEST_DIR"
}

teardown() {
    rm -rf "$TEST_DIR"
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
    run $HK check
    [ "$status" -ne 0 ]

    # Check that the error message contains helpful information
    [[ "$output" =~ "Missing 'amends' declaration" ]] || fail "Should mention missing amends"
    [[ "$output" =~ "Your hk.pkl file should start with one of:" ]] || fail "Should provide examples"
    [[ "$output" =~ "amends \"pkl/Config.pkl\"" ]] || fail "Should show local development example"
    [[ "$output" =~ "amends \"package://github.com/jdx/hk" ]] || fail "Should show package URL example"
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
    run $HK check
    [ "$status" -ne 0 ]

    # Check that the error message contains helpful information
    [[ "$output" =~ "Invalid module URI" ]] || fail "Should mention invalid module URI"
    [[ "$output" =~ "Make sure your 'amends' declaration uses a valid path or package URL" ]] || fail "Should provide guidance"
}

@test "pkl file with syntax errors shows original error" {
    # Create a Pkl config with syntax errors
    cat > hk.pkl << 'EOF'
amends "../pkl/Config.pkl"

hooks = { this is invalid syntax
EOF

    # Run hk and expect it to fail
    run $HK check
    [ "$status" -ne 0 ]

    # Should show the Pkl error (not our custom messages)
    [[ "$output" =~ "Failed to evaluate Pkl config" ]] || [[ "$output" =~ "Unexpected token" ]] || fail "Should show Pkl evaluation error"
}
