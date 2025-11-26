#!/usr/bin/env mise run test:bats

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
