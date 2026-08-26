#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}
teardown() {
    _common_teardown
}

write_project_config() {
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
env { ["HK_TEST_PRECEDENCE"] = "project" }
steps {
    ["project"] { check = "echo project" }
    ["shared"] { check = "echo project-wins" }
}
EOF
    git add hk.pkl
    git commit -m "project config"
}

@test "project environment wins an XDG name collision" {
    write_project_config
    mkdir -p "$HOME/.config/hk"
    cat > "$HOME/.config/hk/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
env { ["HK_TEST_PRECEDENCE"] = "global" }
steps { ["env-source"] { check = "echo env-\$HK_TEST_PRECEDENCE" } }
EOF

    run hk check --all
    assert_success
    assert_output --partial "env-project"
    refute_output --partial "env-global"
}

@test "XDG Config.pkl adds global steps and environment" {
    write_project_config
    mkdir -p "$HOME/.config/hk"
    cat > "$HOME/.config/hk/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
env { ["HK_TEST_GLOBAL"] = "loaded" }
steps { ["global"] { check = "echo global-\$HK_TEST_GLOBAL" } }
EOF

    run hk check --all
    assert_success
    assert_output --partial "project"
    assert_output --partial "global-loaded"
}

@test "project top-level step wins an XDG name collision" {
    write_project_config
    mkdir -p "$HOME/.config/hk"
    cat > "$HOME/.config/hk/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
steps { ["shared"] { check = "echo global-loses" } }
EOF

    run hk check --all
    assert_success
    assert_output --partial "project-wins"
    refute_output --partial "global-loses"
}

@test "XDG Config.pkl can add an explicit hook" {
    write_project_config
    mkdir -p "$HOME/.config/hk"
    cat > "$HOME/.config/hk/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["custom"] {
        steps { ["global-hook"] { check = "echo global-hook" } }
    }
}
EOF

    run hk run custom --all
    assert_success
    assert_output --partial "global-hook"
}

@test "CWD .hkrc.pkl fails with project-local migration guidance" {
    write_project_config
    echo "amends \"$PKL_PATH/Config.pkl\"" > .hkrc.pkl

    run hk check --all
    assert_failure
    assert_output --partial ".hkrc.pkl was removed in hk v2"
    assert_output --partial "hk.local.pkl"
}

@test "HOME .hkrc.pkl fails with XDG migration guidance" {
    write_project_config
    echo "amends \"$PKL_PATH/Config.pkl\"" > "$HOME/.hkrc.pkl"

    run hk check --all
    assert_failure
    assert_output --partial "~/.hkrc.pkl was removed in hk v2"
    assert_output --partial ".config/hk/config.pkl"
}

@test "--hkrc fails with migration guidance" {
    write_project_config

    run hk --hkrc custom.pkl check --all
    assert_failure
    assert_output --partial "--hkrc was removed in hk v2"
    assert_output --partial "hk.local.pkl"
}

@test "UserConfig schema fails with migration guidance" {
    write_project_config
    mkdir -p "$HOME/.config/hk"
    cat > "$HOME/.config/hk/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
environment = new Mapping<String, String> { ["OLD"] = "1" }
EOF

    run hk check --all
    assert_failure
    assert_output --partial "UserConfig.pkl"
    assert_output --partial 'rename `environment` to `env`'
}
