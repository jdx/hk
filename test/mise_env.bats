#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
    if ! command -v mise >/dev/null 2>&1; then
        skip "mise is not installed"
    fi
    # trust mise configs created inside the test repo
    export MISE_TRUSTED_CONFIG_PATHS="$TEST_TEMP_DIR"
}
teardown() {
    _common_teardown
}

@test "HK_MISE resolves mise env for the step dir" {
    export HK_MISE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["toolvar"] {
                dir = "sub"
                glob = "*.txt"
                check = "echo TOOLVAR=\$TOOLVAR; exit 1"
            }
        }
    }
}
EOF
    mkdir -p sub
    cat <<EOF > sub/mise.toml
[env]
TOOLVAR = "from-sub-mise"
EOF
    echo "content" > sub/file.txt
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_failure
    assert_output --partial "TOOLVAR=from-sub-mise"
}

@test "mise env is not resolved without HK_MISE" {
    unset HK_MISE
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["toolvar"] {
                dir = "sub"
                glob = "*.txt"
                check = "echo TOOLVAR=\${TOOLVAR:-unset}; exit 1"
            }
        }
    }
}
EOF
    mkdir -p sub
    cat <<EOF > sub/mise.toml
[env]
TOOLVAR = "from-sub-mise"
EOF
    echo "content" > sub/file.txt
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_failure
    assert_output --partial "TOOLVAR=unset"
}

@test "step env wins over mise env" {
    export HK_MISE=1
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["toolvar"] {
                dir = "sub"
                glob = "*.txt"
                env {
                    ["TOOLVAR"] = "from-step-env"
                }
                check = "echo TOOLVAR=\$TOOLVAR; exit 1"
            }
        }
    }
}
EOF
    mkdir -p sub
    cat <<EOF > sub/mise.toml
[env]
TOOLVAR = "from-sub-mise"
EOF
    echo "content" > sub/file.txt
    git add .
    git commit -m "initial commit"

    run hk check --all
    assert_failure
    assert_output --partial "TOOLVAR=from-step-env"
}
