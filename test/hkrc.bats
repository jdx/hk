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

@test "XDG hook settings apply to hooks materialized from project steps" {
    write_project_config
    mkdir -p "$HOME/.config/hk"
    cat > "$HOME/.config/hk/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
env { ["HK_TEST_HOOK_ONLY"] = "global-config" }
hooks {
    ["check"] {
        env {
            ["HK_TEST_HOOK_ENV"] = "global-hook"
            ["HK_TEST_HOOK_ONLY"] = "global-only"
        }
        report = "touch global-report"
    }
}
steps {
    ["global"] {
        check = "test \"\$HK_TEST_HOOK_ENV\" != global-hook && test \"\$HK_TEST_HOOK_ONLY\" = global-only"
    }
}
EOF
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
env { ["HK_TEST_HOOK_ENV"] = "project-config" }
steps {
    ["project"] {
        check = "test \"\$HK_TEST_HOOK_ENV\" = project-config && test \"\$HK_TEST_HOOK_ONLY\" = global-only"
    }
}
EOF

    run hk check --all
    assert_success
    [ -f global-report ]

    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
steps { ["project"] { check = "test \"\$HK_TEST_HOOK_ENV\" = project-hook" } }
hooks {
    ["check"] {
        env { ["HK_TEST_HOOK_ENV"] = "project-hook" }
        report = "touch project-report"
    }
}
EOF
    rm global-report

    run hk check --all
    assert_success
    [ -f project-report ]
    [ ! -f global-report ]
}

@test "XDG hooks only add to project hooks materialized from top-level steps" {
    mkdir -p "$HOME/.config/hk"
    cat > "$HOME/.config/hk/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
hooks {
    ["pre-commit"] {
        enabled = false
        fix = false
        stash = "none"
        stage = false
        fail_on_fix = true
        report = "touch global-report"
    }
}
EOF
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
steps {
    ["normalize"] {
        glob = "file.txt"
        check = "grep -q good {{files}}"
        fix = "./fix.sh {{files}}"
    }
}
EOF
    cat > fix.sh <<'EOF'
#!/bin/sh
set -eu
test "$(tail -n 1 file.txt)" = staged
sed 's/bad/good/' file.txt > file.txt.new
mv file.txt.new file.txt
EOF
    chmod +x fix.sh
    printf 'bad\nbase\n' > file.txt
    git add hk.pkl fix.sh file.txt
    git commit -m "initial config"
    hk install

    printf 'bad\nstaged\n' > staged-file
    staged_blob=$(git hash-object -w staged-file)
    git update-index --cacheinfo 100644 "$staged_blob" file.txt
    printf 'bad\nstaged\nunstaged\n' > file.txt
    rm staged-file

    run git commit -m "exercise implicit pre-commit"
    assert_success
    [ -f global-report ]
    run git show HEAD:file.txt
    assert_output $'good\nstaged'
    run cat file.txt
    assert_output $'good\nstaged\nunstaged'
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
env { ["HK_TEST_XDG_LAYER"] = "global-config" }
hooks {
    ["custom"] {
        env {
            ["HK_TEST_PRECEDENCE"] = "global-hook"
            ["HK_TEST_XDG_LAYER"] = "global-hook"
        }
        steps {
            ["global-hook"] {
                env {
                    ["HK_TEST_PRECEDENCE"] = "global-step"
                    ["HK_TEST_XDG_LAYER"] = "global-step"
                }
                check = "test \"\$HK_TEST_PRECEDENCE\" = project && echo custom-\$HK_TEST_XDG_LAYER"
            }
        }
    }
}
EOF

    run hk run custom --all
    assert_success
    assert_output --partial "custom-global-step"
}

@test "XDG Config.pkl provides scalar settings" {
    cat > hk.pkl <<EOF
amends "$PKL_PATH/Config.pkl"
EOF

    export HK_CONFIG_DIR="$TEST_TEMP_DIR/.config/hk"
    mkdir -p "$HK_CONFIG_DIR"
    cat > "$HK_CONFIG_DIR/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
stash_backup_count = 0
terminal_progress = false
walk_ignore = false
EOF

    run hk config dump
    assert_success
    [ "$(echo "$output" | jq -r '.stash_backup_count')" = "0" ]
    [ "$(echo "$output" | jq -r '.terminal_progress')" = "false" ]
    [ "$(echo "$output" | jq -r '.walk_ignore')" = "false" ]
}

@test "project-root .hkrc.pkl fails from a subdirectory with migration guidance" {
    write_project_config
    echo "amends \"$PKL_PATH/Config.pkl\"" > .hkrc.pkl
    mkdir nested
    cd nested

    run hk check --all
    assert_failure
    assert_output --partial ".hkrc.pkl was removed in hk v2"
    assert_output --partial "hk.local.pkl"
    assert_output --partial "https://hk.jdx.dev/migration-v2"
}

@test "HOME .hkrc.pkl fails with XDG migration guidance" {
    write_project_config
    echo "amends \"$PKL_PATH/Config.pkl\"" > "$HOME/.hkrc.pkl"

    run hk check --all
    assert_failure
    assert_output --partial "~/.hkrc.pkl was removed in hk v2"
    assert_output --partial ".config/hk/config.pkl"
    assert_output --partial "https://hk.jdx.dev/migration-v2"
}

@test "--hkrc fails with migration guidance" {
    write_project_config

    run hk --hkrc custom.pkl check --all
    assert_failure
    assert_output --partial "--hkrc was removed in hk v2"
    assert_output --partial "hk.local.pkl"
    assert_output --partial "https://hk.jdx.dev/migration-v2"
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
    assert_output --partial "https://hk.jdx.dev/migration-v2"
}

@test "UserConfig defaults block fails with top-level migration guidance" {
    write_project_config
    mkdir -p "$HOME/.config/hk"
    cat > "$HOME/.config/hk/config.pkl" <<EOF
amends "$PKL_PATH/Config.pkl"
defaults { jobs = 2 }
EOF

    run hk check --all
    assert_failure
    assert_output --partial 'move settings from `defaults` to the top level'
    assert_output --partial '`jobs`, `skip_steps`, `skip_hooks`, and `profiles`'
    assert_output --partial "https://hk.jdx.dev/migration-v2"
}

@test "global config deserialization uses missing-amends guidance" {
    write_project_config
    mkdir -p "$HOME/.config/hk"
    echo 'unexpected = 1' > "$HOME/.config/hk/config.pkl"

    run hk check --all
    assert_failure
    assert_output --partial "Missing 'amends' declaration"
    assert_output --partial "$HOME/.config/hk/config.pkl"
}
