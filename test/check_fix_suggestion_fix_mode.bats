#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "no fix suggestion on fix run even if check_first triggers check" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["fix"] {
    steps {
      ["fmt"] {
        check_first = true
        // Failing check
        check = "sh -c 'echo check failed >&2; exit 1'"
        // Define a simple fix command
        fix = "echo fix {{files}}"
      }
    }
  }
}
EOF

    echo "x" > a.js

    run hk fix a.js
    # The overall run is fix; it should not print a suggestion that starts with "To fix, run:"
    refute_output --partial "To fix, run:"
}
