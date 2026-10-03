setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

@test "validate" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
import "$PKL_PATH/Builtins.pkl"
hooks {
    ["pre-commit"] { steps { ["newlines"] = Builtins.newlines } }
    ["pre-push"] { steps { ["newlines"] = Builtins.newlines } }
    ["fix"] { steps { ["newlines"] = Builtins.newlines } }
    ["check"] { steps { ["newlines"] = Builtins.newlines } }
}
EOF
    hk validate
}

@test "validate rejects a dependency cycle instead of hanging" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { depends = List("b"); check = "true" }
            ["b"] { depends = List("c"); check = "true" }
            ["c"] { depends = List("a"); check = "true" }
        }
    }
}
EOF
    run timeout 20 hk validate
    assert_failure
    refute [ "$status" -eq 124 ]
    assert_output --partial "circular dependency"
    assert_output --partial "hook 'check'"
    assert_output --partial "a -> b -> c -> a"
    # The same config used to hang `hk check` forever.
    run timeout 20 hk check --all
    assert_failure
    refute [ "$status" -eq 124 ]
    assert_output --partial "circular dependency"
}

@test "validate rejects a step that depends on itself" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] { depends = List("lint"); check = "true" }
        }
    }
}
EOF
    run timeout 20 hk validate
    assert_failure
    refute [ "$status" -eq 124 ]
    assert_output --partial "Step 'lint' in hook 'check' depends on itself"
}

@test "validate accepts depends across execution groups" {
    # depends only orders steps within one group, so a name in a later group is not a cycle.
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { depends = List("b"); check = "true" }
            ["b"] { exclusive = true; depends = List("a"); check = "true" }
        }
    }
}
EOF
    run timeout 20 hk validate
    assert_success
}

@test "validate names the step for an invalid glob" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] { glob = "src/[abc"; check = "true" }
        }
    }
}
EOF
    run hk validate
    assert_failure
    assert_output --partial "Step 'lint' in hook 'check'"
    assert_output --partial "invalid glob 'src/[abc'"
}

@test "validate names the step for an invalid exclude regex" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] { glob = "*"; exclude = Regex("vendor/("); check = "true" }
        }
    }
}
EOF
    run hk validate
    assert_failure
    assert_output --partial "Step 'lint' in hook 'check'"
    assert_output --partial "invalid exclude"
    assert_output --partial "vendor/("
}
