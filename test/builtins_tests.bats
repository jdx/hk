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
    // Versioned and strict variants are exercised separately below.
    steps = Builtins.all
  }
}
PKL

    # The Bats task preinstalls runtimes before parallel test execution starts.
    export JAVA_HOME="$(mise where java@21)"
    # Prepend so stub-pinned tools take precedence over any ambient tools
    # preinstalled on the runner (e.g. ubuntu-latest ships a global tsc).
    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$JAVA_HOME/bin:$PATH"
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
      ["gitleaks"] = (Builtins.gitleaks) {
        scan = "staged"
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

@test "editorconfig-checker builtin tests run with editorconfig-checker v4" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["editorconfig_checker"] = Builtins.editorconfig_checker
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step editorconfig_checker
    assert_success
    assert_output --partial "ok - editorconfig_checker :: check bad file"
    assert_output --partial "ok - editorconfig_checker :: check good file"
}

@test "editorconfig-checker v3 builtin tests run with ec" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["editorconfig_checker_v3"] = (Builtins.editorconfig_checker) {
        version = "3"
      }
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step editorconfig_checker_v3
    assert_success
    assert_output --partial "ok - editorconfig_checker_v3 :: check bad file"
    assert_output --partial "ok - editorconfig_checker_v3 :: check good file"
}

@test "pinact v3 builtin tests run with pinact v3" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["pinact_v3"] = (Builtins.pinact) {
        version = "3"
      }
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs_v3:$PATH"
    run hk test --step pinact_v3
    assert_success
    assert_output --partial "ok - pinact_v3 :: fix bad file and mismatched version comment"
}

@test "knip strict option tests run" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["knip_strict"] = (Builtins.knip) {
        strict = true
      }
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$(mise where node@latest)/bin:$PATH"
    run hk test --step knip_strict
    assert_success
    assert_output --partial "ok - knip_strict :: check bad file"
}

@test "kubeconform dirs option retargets the glob and its tests" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["kubeconform_apps"] = (Builtins.kubeconform) {
        dirs = List("apps")
      }
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step kubeconform_apps
    assert_success
    assert_output --partial "ok - kubeconform_apps :: check manifest without kind"
    assert_output --partial "ok - kubeconform_apps :: check skips helm values and kustomization"
}

@test "kubeconform test_dir keeps the tests valid when glob is replaced" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["kubeconform_gitops"] = (Builtins.kubeconform) {
        glob = Regex(#"^gitops/apps/[^/]+/[^/]+\.yaml$"#)
        test_dir = "gitops/apps/demo/"
      }
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step kubeconform_gitops
    assert_success
    assert_output --partial "ok - kubeconform_gitops :: check manifest without kind"
}

@test "kubeconform extra_excludes adds to the default excludes" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["kubeconform_extra"] = (Builtins.kubeconform) {
        extra_excludes = List("**/custom.yaml")
        tests {
          ["check skips extra and default excludes"] {
            run = "check"
            tmpdir = true
            write {
              ["k8s/resource.yaml"] = "# no resources here\n"
              ["k8s/custom.yaml"] = "key: value\n"
              ["k8s/values.yaml"] = "key: value\n"
              ["k8s/kustomization.yaml"] = "key: value\n"
            }
            expect { code = 0 }
          }
        }
      }
    }
  }
}
PKL

    PATH="$PROJECT_ROOT/test/builtin_tool_stubs:$PATH"
    run hk test --step kubeconform_extra
    assert_success
    assert_output --partial "ok - kubeconform_extra :: check skips extra and default excludes"
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

@test "ruff builtins select extensionless python scripts" {
    cat <<PKL > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl" as Builtins
hooks {
  ["check"] {
    steps {
      ["ruff"] = (Builtins.ruff) {
        check = "for f in {{ files }}; do echo ruff:\$f; done"
      }
      ["ruff_format"] = (Builtins.ruff_format) {
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
