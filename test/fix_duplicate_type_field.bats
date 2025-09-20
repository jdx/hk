#!/usr/bin/env bats

# Test for https://github.com/jdx/hk/discussions/296
# Verifies that duplicate _type fields are not generated in JSON when using groups

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "basic group configuration works" {
    # First, test that a basic config without groups works
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["step1"] {
        shell = "echo step1"
      }
    }
  }
}
EOF

    run hk validate
    assert_success
}

@test "group configuration validates without duplicate _type error" {
    # Create a config file with groups like in the discussion
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["pre-commit"] {
    steps {
      ["frontend"] = new Group {
        steps {
          ["prettier"] {
            shell = "prettier --check"
            glob = List("*.js", "*.jsx", "*.ts", "*.tsx")
          }
          ["eslint"] {
            shell = "eslint"
            glob = List("*.js", "*.jsx", "*.ts", "*.tsx")
          }
        }
      }
      ["backend"] = new Group {
        steps {
          ["black"] {
            shell = "black --check"
            glob = List("*.py")
          }
        }
      }
    }
  }
}
EOF

    # Validate should succeed without duplicate _type field errors
    run hk validate
    assert_success

    # Check the output doesn't contain the duplicate field error
    refute_output --partial "duplicate field \`_type\`"
    refute_output --partial "failed to parse cache file"
}

@test "loading cached config with groups doesn't cause duplicate _type" {
    # Create the same config
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["my_group"] = new Group {
        steps {
          ["step1"] {
            shell = "echo hello"
          }
        }
      }
    }
  }
}
EOF

    # Run validate first time to generate cache
    run hk validate
    assert_success
    refute_output --partial "duplicate field \`_type\`"

    # Run validate second time to load from cache
    run hk validate
    assert_success
    refute_output --partial "duplicate field \`_type\`"
    refute_output --partial "failed to parse cache file"

    # Run check to trigger loading and using the config
    echo "test" > test.txt
    run hk check --all
    refute_output --partial "duplicate field \`_type\`"
    refute_output --partial "failed to parse cache file"
}
