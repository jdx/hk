#!/usr/bin/env bats

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

@test "check failure with check_diff filters files in suggestion when list-files also exists" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fmt"] {
        check_diff = "sh -c 'printf \"%s\n\" \"--- a.js\" \"+++ a.js\" \"@@ -1 +1 @@\" \"-bad\" \"+good\"; exit 1'"
        check_list_files = "sh -c 'echo b.js; exit 1'"
        fix = "echo fix {{files}}"
      }
    }
  }
}
EOF

    echo "bad" > a.js
    echo "good" > b.js

    run hk check a.js b.js
    assert_failure
    assert_output --partial "To fix, run: echo fix a.js"
    refute_output --partial "To fix, run: echo fix a.js b.js"
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

@test "check_failed_files runs detailed check only on listed files" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] {
        check_list_files = "sh -c 'echo a.js; exit 1'"
        check = "sh -c 'echo detailed:{{files}} >&2; exit 1'"
        fix = "echo fix {{files}}"
        check_failed_files = true
      }
    }
  }
}
EOF

    echo "bad" > a.js
    echo "good" > b.js

    run hk check a.js b.js
    assert_failure
    assert_output --partial "detailed:a.js"
    refute_output --partial "detailed:a.js b.js"
    assert_output --partial "To fix, run: echo fix a.js"
}

@test "check_failed_files keeps listing failure authoritative" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] {
        check_list_files = "sh -c 'echo a.js; exit 1'"
        check = "echo focused {{files}}"
        check_failed_files = true
      }
    }
  }
}
EOF

    echo "bad" > a.js

    run hk check a.js
    assert_failure
    assert_output --partial "file-listing check failed but focused check succeeded"
}

@test "check_failed_files preserves listing diagnostics when focused check fails" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] {
        check_list_files = "sh -c 'echo a.js; echo listing-diagnostic >&2; exit 1'"
        check = "sh -c 'echo focused-diagnostic >&2; exit 1'"
        check_failed_files = true
      }
    }
  }
}
EOF

    echo "bad" > a.js

    run hk check a.js
    assert_failure
    assert_output --partial "listing-diagnostic"
    assert_output --partial "focused-diagnostic"
}

@test "long fix suggestion falls back to focused hk command" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["fmt"] {
        check = "exit 1"
        fix = "echo fix {{files}}"
      }
    }
  }
}
EOF

    files=()
    for i in $(seq 1 400); do
        file="long-filename-for-output-$i.js"
        echo "bad" > "$file"
        files+=("$file")
    done

    run hk check "${files[@]}"
    assert_failure
    assert_output --partial "To fix, run: hk fix -S fmt"
    refute_output --partial "To fix, run: echo fix"
}

@test "check_failed_files validates required commands" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
  ["check"] {
    steps {
      ["lint"] {
        check = "exit 1"
        check_failed_files = true
      }
    }
  }
}
EOF

    run hk validate
    assert_failure
    assert_output --partial \
        "check_failed_files = true\` requires \`check\` and at least one of \`check_diff\` or \`check_list_files"
}
