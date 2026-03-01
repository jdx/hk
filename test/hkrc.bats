#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

@test "hkrc: loads default .hkrc.pkl from project directory" {
    # Create a basic project config
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["echo"] { check = "echo 'project config'" }
        }
    }
}
EOF

    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

environment {
    ["HK_TEST_VAR"] = "from_hkrc"
}

hooks {
    ["pre-commit"] {
        environment {
            ["HOOK_VAR"] = "hook_value"
        }
        steps {
            ["echo"] {
                environment {
                    ["STEP_VAR"] = "step_value"
                }
            }
        }
    }
}
EOF

    git add hk.pkl .hkrc.pkl
    git commit -m "initial commit"

    # Run the hook and verify environment variables are set
    run hk run pre-commit --all
    assert_success
}

@test "hkrc: custom path with --hkrc flag" {
    # Create a basic project config
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["env_test"] {
                check = "env | grep CUSTOM_VAR || echo 'CUSTOM_VAR not found'"
            }
        }
    }
}
EOF

    # Create a custom user config
    cat <<EOF > custom.hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

environment {
    ["CUSTOM_VAR"] = "custom_value"
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    # Run with custom config
    run hk --hkrc custom.hkrc.pkl run pre-commit --all
    assert_success
}

@test "hkrc: fails when custom config file doesn't exist" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["echo"] { check = "echo test" }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    # Try to use non-existent config
    run hk --hkrc nonexistent.pkl run pre-commit --all
    assert_failure
    assert_output --partial "Config file not found"
}

@test "hkrc: per-hook environment variables" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["check_hook_var"] {
                check = "test \"\$HOOK_SPECIFIC_VAR\" = \"hook_value\" && echo 'Hook var found'"
            }
        }
    }
    ["check"] {
        steps {
            ["check_no_hook_var"] {
                check = "test -z \"\$HOOK_SPECIFIC_VAR\" && echo 'Hook var not found (correct)'"
            }
        }
    }
}
EOF

    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

hooks {
    ["pre-commit"] {
        environment {
            ["HOOK_SPECIFIC_VAR"] = "hook_value"
        }
    }
}
EOF

    git add hk.pkl .hkrc.pkl
    git commit -m "initial commit"

    # Test pre-commit hook has the variable
    run hk run pre-commit --all
    assert_success
    assert_output --partial "Hook var found"

    # Test pre-push hook doesn't have the variable
    run hk run check --all
    assert_success
    assert_output --partial "Hook var not found (correct)"
}

@test "hkrc: per-step configuration overrides" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["test_glob"] {
                glob = "*.txt"
                check = "echo 'Found files:' && echo {{ files }}"
            }
        }
    }
}
EOF

    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["test_glob"] {
                glob = "*.py"
            }
        }
    }
}
EOF

    # Create test files
    echo "test" > test.txt
    echo "test" > test.py

    git add hk.pkl .hkrc.pkl test.txt test.py
    git commit -m "initial commit"

    # Modify files to trigger the hook
    echo "modified" > test.txt
    echo "modified" > test.py
    git add test.txt test.py

    # The user config should override glob to only match .py files
    run hk run pre-commit
    assert_success
    assert_output --partial "test.py"
    refute_output --partial "test.txt"
}

@test "hkrc: global environment variables" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["check_global"] {
                check = "test \"\$GLOBAL_TEST_VAR\" = \"global_value\" && echo 'Global var found'"
            }
        }
    }
}
EOF

    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

environment {
    ["GLOBAL_TEST_VAR"] = "global_value"
}
EOF

    git add hk.pkl .hkrc.pkl
    git commit -m "initial commit"

    run hk run pre-commit --all
    assert_success
    assert_output --partial "Global var found"
}

@test "hkrc: user config takes precedence over project config for environment" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
env {
    ["PRECEDENCE_TEST"] = "project_value"
}
hooks {
    ["pre-commit"] {
        steps {
            ["check_precedence"] {
                check = "test \"\$PRECEDENCE_TEST\" = \"user_value\" && echo 'User config wins'"
            }
        }
    }
}
EOF

    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

environment {
    ["PRECEDENCE_TEST"] = "user_value"
}
EOF

    git add hk.pkl .hkrc.pkl
    git commit -m "initial commit"

    run hk run pre-commit --all
    assert_success
    assert_output --partial "User config wins"
}

@test "hkrc: validate command works with user config" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["echo"] { check = "echo test" }
        }
    }
}
EOF

    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

environment {
    ["TEST_VAR"] = "test_value"
}
EOF

    git add hk.pkl .hkrc.pkl
    git commit -m "initial commit"

    run hk validate
    assert_success
    assert_output --partial "is valid"
}

@test "hkrc: Config-format hkrc with steps runs without panic" {
    # Reproduces the docs' own hkrc example (docs/configuration.md lines 183-201).
    # The hkrc amends Config.pkl (not UserConfig.pkl) and defines hooks with
    # steps that have check/fix/glob — fields that exist on Step but not on
    # UserConfig's HookConfig. hk currently deserializes hkrc as UserConfig,
    # where "check" is Option<bool>, so any string command panics.
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["trailing-whitespace"] { check = "echo 'project step'" }
        }
    }
}
EOF

    # Mirrors the docs example: hkrc with check/fix on steps
    cat <<EOF > my-hkrc.pkl
amends "$PKL_PATH/Config.pkl"

hooks {
    ["pre-commit"] {
        fix = true
        steps {
            ["eslint"] {
                check = "echo 'eslint check'"
                fix = "echo 'eslint fix'"
            }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    run hk --hkrc my-hkrc.pkl run pre-commit --all
    assert_success
    assert_output --partial "project step"
    assert_output --partial "eslint"
}

@test "hkrc: Config-format hkrc adds hook the project doesn't have" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["echo"] { check = "echo 'project pre-commit'" }
        }
    }
}
EOF

    cat <<EOF > my-hkrc.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["hkrc-only"] { check = "echo 'hkrc check hook'" }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    # The hkrc adds the "check" hook that the project doesn't define
    run hk --hkrc my-hkrc.pkl run check --all
    assert_success
    assert_output --partial "hkrc check hook"
}

@test "hkrc: project step overrides same-named hkrc step" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["shared-step"] { check = "echo 'project wins'" }
        }
    }
}
EOF

    cat <<EOF > my-hkrc.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["shared-step"] { check = "echo 'hkrc loses'" }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    # When both define the same step, project should win
    run hk --hkrc my-hkrc.pkl run pre-commit --all
    assert_success
    assert_output --partial "project wins"
    refute_output --partial "hkrc loses"
}

@test "hkrc: merges different steps from same hook" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["project-step"] { check = "echo 'from project'" }
        }
    }
}
EOF

    cat <<EOF > my-hkrc.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["hkrc-step"] { check = "echo 'from hkrc'" }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    # Both steps should run — they have different names so they merge
    run hk --hkrc my-hkrc.pkl run pre-commit --all
    assert_success
    assert_output --partial "from project"
    assert_output --partial "from hkrc"
}

@test "hkrc: default path loads from home directory" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["echo"] { check = "echo 'project step'" }
        }
    }
}
EOF

    # HOME is set to TEST_TEMP_DIR by common_setup
    cat <<EOF > "$HOME/.hkrc.pkl"
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["home-step"] { check = "echo 'from home hkrc'" }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    # Without --hkrc, should discover ~/.hkrc.pkl and merge it
    run hk run pre-commit --all
    assert_success
    assert_output --partial "from home hkrc"
    assert_output --partial "project step"
}

@test "hkrc: XDG config path loads from config dir" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["echo"] { check = "echo 'project step'" }
        }
    }
}
EOF

    # Place hkrc in the XDG config dir
    export HK_CONFIG_DIR="$TEST_TEMP_DIR/.config/hk"
    mkdir -p "$HK_CONFIG_DIR"
    cat <<EOF > "$HK_CONFIG_DIR/config.pkl"
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["xdg-step"] { check = "echo 'from xdg config'" }
        }
    }
}
EOF

    git add hk.pkl
    git commit -m "initial commit"

    # Without --hkrc and no ~/.hkrc.pkl, should discover XDG config
    run hk run pre-commit --all
    assert_success
    assert_output --partial "from xdg config"
    assert_output --partial "project step"
}

@test "hkrc: step exclude patterns from user config" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] {
        steps {
            ["test_exclude"] {
                glob = "*.py"
                check = "echo 'Processing:' && echo {{ files }}"
            }
        }
    }
}
EOF

    cat <<EOF > .hkrc.pkl
amends "$PKL_PATH/UserConfig.pkl"

hooks {
    ["pre-commit"] {
        steps {
            ["test_exclude"] {
                exclude = "test_*.py"
            }
        }
    }
}
EOF

    # Create test files
    echo "test" > main.py
    echo "test" > test_example.py

    git add hk.pkl .hkrc.pkl main.py test_example.py
    git commit -m "initial commit"

    # Modify files to trigger the hook
    echo "modified" > main.py
    echo "modified" > test_example.py
    git add main.py test_example.py

    # The user config should exclude test_*.py files
    run hk run pre-commit
    assert_success
    assert_output --partial "main.py"
    refute_output --partial "test_example.py"
}
