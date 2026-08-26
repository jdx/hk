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

@test "builtin factories support stable values, options, nested step overrides, and all" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"

hooks {
    ["check"] {
        steps {
            ["prettier"] = Builtins.prettier
            ["gitleaks"] = (Builtins.gitleaks) {
                staged = true
                step { batch = false }
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

    run env HK_PKL_BACKEND=pkl hk validate
    assert_success
}

@test "pkl CLI backend can still be selected" {
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

    run env HK_PKL_BACKEND=pkl hk check --all
    assert_success
    assert_file_exists ran.txt
}

@test "unknown pkl backend warns and uses pklr" {
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

    run env HK_PKL_BACKEND=pkrl hk check --all
    assert_success
    assert_output --partial 'unrecognized HK_PKL_BACKEND value "pkrl"'
    assert_file_exists ran.txt
}
