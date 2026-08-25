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
    // Include all builtin steps except versioned builtins that require a
    // different tool stub. Those are exercised separately below.
    steps =
      Builtins.all
        .toMap()
        .filter((name, _) -> name != "pinact_v3")
        .toMapping()
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

@test "gitleaks staged option tests run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["gitleaks"] = (Builtins.gitleaks()) {
        staged = true
      }
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step gitleaks
    assert_success
    assert_output --partial "ok - gitleaks :: check bad staged file"
}

@test "pinact v3 builtin tests run with pinact v3" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["pinact_v3"] = Builtins.pinact_v3()
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs_v3:$PATH"
    run hk test --step pinact_v3
    assert_success
    assert_output --partial "ok - pinact_v3 :: fix bad file and mismatched version comment"
}

@test "shell builtins select extensionless sh scripts but not fish" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["shellcheck"] = (Builtins.shellcheck()) {
        check = "echo shellcheck {{ files }}"
      }
      ["shfmt"] = (Builtins.shfmt()) {
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

@test "ruff builtins select extensionless python scripts" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["ruff"] = (Builtins.ruff()) {
        check = "for f in {{ files }}; do echo ruff:\$f; done"
      }
      ["ruff_format"] = (Builtins.ruff_format()) {
        check = "for f in {{ files }}; do echo ruff_format:\$f; done"
      }
    }
  }
}
PKL

    echo "print('python')" > test.py
    cat <<'SCRIPT' > script
#!/usr/bin/env python
print('python')
SCRIPT
    chmod +x script
    echo "console.log('javascript')" > test.js

    run hk check --all
    assert_success
    assert_output --partial "ruff:script"
    assert_output --partial "ruff:test.py"
    assert_output --partial "ruff_format:script"
    assert_output --partial "ruff_format:test.py"
    refute_output --partial "test.js"
}
