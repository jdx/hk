#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "default pklr backend can evaluate a basic config" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["echo"] {
                check = "echo ok > ran.txt"
            }
        }
    }
}
EOF

    run hk check --all
    assert_success
    assert_file_exists ran.txt
}

@test "pklr backend validates config" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["step1"] { check = "echo step1" }
        }
    }
}
EOF

    run hk validate
    assert_success
}

@test "default pklr backend can evaluate a group" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["group"] = new Group {
                steps {
                    ["echo"] {
                        check = "echo ok > group-ran.txt"
                    }
                }
            }
        }
    }
}
EOF

    run hk check --all
    assert_success
    assert_file_exists group-ran.txt
}

@test "builtin step subclasses support stable values, options, direct overrides, and all" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["prettier"] = Builtins.prettier
            ["gitleaks"] = (Builtins.gitleaks) {
                scan = "staged"
                batch = false
            }
            ["editorconfig_checker_v3"] = (Builtins.editorconfig_checker) {
                version = "3"
            }
            ["all"] = new Group {
                steps = Builtins.all
            }
        }
    }
}
EOF

    run hk validate
    assert_success
}

@test "a step test resolves a local before the StepTest property of the same name when the step is amended" {
    # `after` is both a local here and a nullable `StepTest` property. Pkl
    # resolves the local, so the test writes and expects "formatted".
    cat <<EOF > formatter.pkl
import "$PKL_PATH/Config.pkl"

formatter = new Config.Step {
  glob = "*.txt"
  check = "grep -qx formatted {{ files }}"
  tests {
    local const after = "formatted\n"
    ["check formatted file"] {
      tmpdir = true
      write { ["a.txt"] = after }
      expect { files { ["a.txt"] = after } }
    }
  }
}
EOF
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "formatter.pkl"

hooks {
    ["check"] {
        steps {
            ["formatter"] = (formatter.formatter) { exclude = "vendor/**" }
        }
    }
}
EOF

    run hk validate
    assert_success
    run hk test --step formatter
    assert_success
    assert_output --partial "ok - formatter :: check formatted file"
}
