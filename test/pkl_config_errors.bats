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

@test "syntax error in hk.pkl reports file, line and column once" {
    cat > hk.pkl <<'EOF'
amends "../pkl/Config.pkl"

hooks {
  ["check"] {
    steps { ["a"] { glob = = "*.rs" } }
  }
}
EOF
    run hk check
    assert_failure
    assert_output --partial "Failed to evaluate Pkl config"
    assert_output --partial "hk.pkl:5:"
    assert_output --partial "unexpected token in expression"
    assert_output --partial 'steps { ["a"] { glob = = "*.rs" } }'
    # The config path appears once, in the located message.
    [ "$(grep -o 'hk.pkl' <<<"$output" | wc -l)" -eq 1 ]
}

@test "syntax error in an imported file is blamed on that file" {
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
import "./steps.pkl" as S
hooks { ["check"] { steps { ["a"] = S.a } } }
EOF
    cat > steps.pkl <<'EOF'
a {
  check = "true"
  glob = = "*.rs"
}
EOF
    run hk check
    assert_failure
    assert_output --partial "steps.pkl:3:"
    assert_output --partial 'glob = = "*.rs"'
    refute_output --partial "hk.pkl:"
    refute_output --partial "./steps.pkl"
}
