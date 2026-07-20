#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "builtins tests run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    // Include all Builtins.* steps
    steps = Builtins.toMap().toMapping()
  }
}
PKL

    # Prepend so stub-pinned tools take precedence over any ambient tools
    # preinstalled on the runner (e.g. ubuntu-latest ships a global tsc).
    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test
    assert_success
    # At least the newlines builtin has a test
    assert_output --partial "ok - newlines :: fix bad file"
}

@test "shell builtins select extensionless sh scripts but not fish" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["shellcheck"] = (Builtins.shellcheck) {
        check = "echo shellcheck {{ files }}"
      }
      ["shfmt"] = (Builtins.shfmt) {
        check = "echo shfmt {{ files }}"
      }
    }
  }
}
PKL

    cat <<'SCRIPT' > script
#!/bin/sh
echo shell
SCRIPT
    cat <<'SCRIPT' > fish-script
#!/usr/bin/env fish
echo fish
SCRIPT

    run hk check --all
    assert_success
    assert_output --partial "shellcheck script"
    assert_output --partial "shfmt script"
    refute_output --partial "fish-script"
}
