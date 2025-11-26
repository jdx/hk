#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "hk finds config in .config/hk.pkl" {
    cd "$BATS_TEST_TMPDIR"
    git init

    # Create .config directory and hk.pkl inside it
    mkdir -p .config
    cat > .config/hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test"] {
        check = "echo 'found config in .config/hk.pkl'"
      }
    }
  }
}
EOF

    # hk check should find and use the config
    run hk check
    assert_success
    assert_output --partial "found config in .config/hk.pkl"
}

@test "hk.pkl takes precedence over .config/hk.pkl" {
    cd "$BATS_TEST_TMPDIR"
    git init

    # Create both hk.pkl and .config/hk.pkl
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test"] {
        check = "echo 'using hk.pkl'"
      }
    }
  }
}
EOF

    mkdir -p .config
    cat > .config/hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test"] {
        check = "echo 'using .config/hk.pkl'"
      }
    }
  }
}
EOF

    # hk check should use hk.pkl (higher precedence)
    run hk check
    assert_success
    assert_output --partial "using hk.pkl"
    refute_output --partial "using .config/hk.pkl"
}

@test ".config/hk.pkl is found when hk.pkl doesn't exist" {
    cd "$BATS_TEST_TMPDIR"
    git init

    # Create only .config/hk.pkl (not hk.pkl)
    mkdir -p .config
    cat > .config/hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test_step"] {
        check = "echo 'success from .config'"
      }
    }
  }
}
EOF

    # Verify hk.pkl doesn't exist
    [[ ! -f hk.pkl ]]

    # hk check should find .config/hk.pkl
    run hk check
    assert_success
    assert_output --partial "success from .config"
}

@test ".config/hk.pkl works in subdirectories" {
    cd "$BATS_TEST_TMPDIR"
    git init

    # Create .config/hk.pkl in project root
    mkdir -p .config
    cat > .config/hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test"] {
        check = "echo 'config found from subdirectory'"
      }
    }
  }
}
EOF

    # Create a subdirectory and run hk from there
    mkdir -p subdir/nested
    cd subdir/nested

    # hk check should find .config/hk.pkl from parent directory
    run hk check
    assert_success
    assert_output --partial "config found from subdirectory"
}

@test "HK_FILE set to custom path does not fall back to .config/hk.pkl" {
    cd "$BATS_TEST_TMPDIR"
    git init

    # Create .config/hk.pkl (should NOT be used when HK_FILE is set)
    mkdir -p .config
    cat > .config/hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test"] {
        check = "echo 'WRONG: using .config/hk.pkl'"
      }
    }
  }
}
EOF

    # Create custom config file at the HK_FILE path
    cat > custom-config.pkl <<EOF
amends "$PKL_PATH/Config.pkl"

hooks {
  ["check"] {
    steps {
      ["test"] {
        check = "echo 'CORRECT: using custom-config.pkl'"
      }
    }
  }
}
EOF

    # Set HK_FILE to the custom path
    export HK_FILE="custom-config.pkl"

    # hk check should use custom-config.pkl, NOT .config/hk.pkl
    run hk check
    assert_success
    assert_output --partial "CORRECT: using custom-config.pkl"
    refute_output --partial "WRONG: using .config/hk.pkl"
}
