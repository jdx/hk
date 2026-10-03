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
