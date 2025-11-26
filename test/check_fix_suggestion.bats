#!/usr/bin/env mise run test:bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "check failure suggests short one-line fix command" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fmt"] {
        // Failing check
        check = "sh -c 'echo check failed >&2; exit 1'"
        // Short one-line fix using files list
        fix = "echo fix {{files}}"
      }
    }
  }
}
EOF

    echo "x" > a.js
    echo "y" > b.js

    run hk check a.js b.js
    assert_failure
    assert_output --partial "To fix, run: echo fix a.js b.js"
}

@test "check failure with list-files filters files in suggestion" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fmt"] {
        // Emits only the first file, then fails
        check_list_files = "sh -c 'echo a.js; exit 1'"
        fix = "echo fix {{files}}"
      }
    }
  }
}
EOF

    echo "x" > a.js
    echo "y" > b.js

    run hk check a.js b.js
    assert_failure
    # Suggestion should include only a.js
    assert_output --partial "To fix, run: echo fix a.js"
    # And should not include b.js
    refute_output --partial "b.js"
}

@test "check failure with multi-line fix suggests hk fix command" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fmt"] {
        check = "sh -c 'echo nope >&2; exit 1'"
        // Multi-line fix command renders >1 line
        fix = "echo line1\n echo line2 {{files}}"
      }
    }
  }
}
EOF

    echo "x" > a.js

    run hk check a.js
    assert_failure
    assert_output --partial "To fix, run: hk fix -S fmt"
    # Should not print the multi-line fix body
    refute_output --partial "echo line1"
}
