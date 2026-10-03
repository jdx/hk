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

# `timeout` is not on macOS; perl's alarm kills a hang with SIGALRM (status 142).
_timeout() {
    perl -e 'alarm shift; exec @ARGV' "$@"
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
    run _timeout 20 hk validate
    assert_failure
    refute [ "$status" -eq 142 ]
    assert_output --partial "circular dependency"
    assert_output --partial "hook 'check'"
    assert_output --partial "a -> b -> c -> a"
    # The same config used to hang `hk check` forever.
    run _timeout 20 hk check --all
    assert_failure
    refute [ "$status" -eq 142 ]
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
    run _timeout 20 hk validate
    assert_failure
    refute [ "$status" -eq 142 ]
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
    run _timeout 20 hk validate
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

@test "validate warns about depends that never orders anything" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["first"] { depends = List("lter", "nowhere"); check = "true" }
            ["later"] { exclusive = true; check = "true" }
        }
    }
}
EOF
    run hk validate
    assert_success
    assert_output --partial "Step 'first' in hook 'check' depends on unknown step 'lter'. Did you mean 'later'?"
    assert_output --partial "depends on unknown step 'nowhere'."
    assert_output --partial "is valid, with 2 warning(s)"
}

@test "validate warns about group names, command-less steps and unmatchable globs" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] { depends = List("grp"); glob = "./src/*.js"; check = "true" }
            ["comma"] { glob = "*.js,*.ts"; check = "true" }
            ["negated"] { glob = "!vendor/**"; check = "true" }
            ["rooted"] { glob = "/src/*.js"; check = "true" }
            ["empty"] { glob = "*.js" }
            ["grp"] = new Group { steps { ["inner"] { check = "true" } } }
        }
    }
}
EOF
    run hk validate
    assert_success
    assert_output --partial "depends on 'grp', which is a group"
    assert_output --partial "Step 'empty' in hook 'check' has no check, fix, check_list_files or check_diff command"
    assert_output --partial "glob './src/*.js'"
    assert_output --partial "glob '*.js,*.ts'"
    assert_output --partial "glob '!vendor/**'"
    assert_output --partial "glob '/src/*.js'"
}

@test "validate warns about a misspelled property" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["lint"] { chek = "true" }
        }
    }
}
EOF
    run hk validate
    if [[ "$output" == *"unknown field"* ]]; then
        # Debug builds reject unknown properties while loading, before the
        # lint runs; release builds drop them and warn.
        assert_output --partial "unknown field `chek`"
    else
        assert_success
        assert_output --partial "unknown property 'chek' in step 'lint' in hook 'check'; hk ignores it. Did you mean 'check'?"
    fi
}

@test "validate accepts a clean config without warnings" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["a"] { check = "true" }
            ["b"] { depends = List("a"); glob = "*.js"; check = "true" }
        }
    }
}
EOF
    run hk validate
    assert_success
    refute_output --partial "WARN"
    assert_output --partial "is valid"
}

@test "validate rejects a min_hk_version newer than hk when written without a patch" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
min_hk_version = "999.0"
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    run hk validate
    assert_failure
    assert_output --partial "minimum required version"
}

@test "validate rejects a min_hk_version with a v prefix" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
min_hk_version = "v999.0.0"
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    run hk validate
    assert_failure
    assert_output --partial "minimum required version"
}

@test "validate tolerates the unrendered min_hk_version placeholder" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
min_hk_version = "{{version | truncate(length=1)}}.0.0"
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    run hk validate
    assert_success
    refute_output --partial "WARN"
}

@test "validate lints top-level steps once, by their real names" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
steps {
    ["lint"] { depends = List("fmt"); check = "true" }
    ["fmt"] { check = "true" }
    ["other"] { depends = List("nope"); check = "true" }
}
EOF
    run hk validate
    assert_success
    assert_output --partial "Step 'other' in hook 'the default hooks' depends on unknown step 'nope'."
    refute_output --partial "Step ''"
    # `fmt` shares lint's group, so that dependency is fine.
    refute_output --partial "'fmt'"
    [ "$(grep -c "unknown step 'nope'" <<<"$output")" -eq 1 ]
}

@test "validate checks a group-level exclude glob" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["grp"] = new Group {
                exclude = "./vendor/**"
                steps { ["inner"] { check = "true" } }
            }
        }
    }
}
EOF
    run hk validate
    assert_success
    assert_output --partial "glob './vendor/**'"
}

@test "validate warns about a min_hk_version with a garbled placeholder" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
min_hk_version = "v999.0.0{{typo}}"
hooks { ["check"] { steps { ["a"] { check = "true" } } } }
EOF
    run hk validate
    assert_output --partial "ignoring min_hk_version"
}
