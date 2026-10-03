#!/usr/bin/env bats

setup() {
    load 'test_helper/common_setup'
    _common_setup
}

teardown() {
    _common_teardown
}

assert_generator_snapshot() {
    local group="$1"
    local target="$2"
    run hk agent "$group" --target "$target"
    assert_success
    assert_output "$(<"$PROJECT_ROOT/test/snapshots/agent/$group-$target.txt")"
}

@test "agent instruction generators match snapshots" {
    assert_generator_snapshot instructions codex
    assert_generator_snapshot instructions claude-code
    assert_generator_snapshot instructions generic
}

@test "agent hook generators match snapshots" {
    assert_generator_snapshot hooks codex
    assert_generator_snapshot hooks claude-code
    assert_generator_snapshot hooks vscode
}

@test "agent MCP generators match snapshots" {
    assert_generator_snapshot mcp codex
    assert_generator_snapshot mcp claude-desktop
    assert_generator_snapshot mcp claude-code
    assert_generator_snapshot mcp vscode
}

@test "agent generators do not edit host configuration" {
    before="$(find . -type f -print | LC_ALL=C sort)"
    hk agent instructions --target codex >/dev/null
    hk agent hooks --target claude-code >/dev/null
    hk agent mcp --target vscode >/dev/null
    after="$(find . -type f -print | LC_ALL=C sort)"
    assert_equal "$after" "$before"
}

write_stop_hook_config() {
    local command="$1"
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["probe"] {
                check = new CommandSpec { command = "$command"; effect = "read" }
            }
        }
    }
}
EOF
}

@test "agent hooks do not print hk run_result JSON" {
    run hk agent hooks --target claude-code
    assert_success
    assert_output --partial '"command": "hk agent stop-hook --target claude-code --timeout 570"'
    assert_output --partial '"timeout": 600'
    refute_output --partial "--format json"
    run hk agent hooks --target codex
    assert_success
    assert_output --partial '"command": "hk agent stop-hook --target codex"'
}

@test "agent stop-hook is silent and exits 0 when the check passes" {
    write_stop_hook_config "true"
    run bash -c "echo '{\"stop_hook_active\":false}' | hk agent stop-hook"
    assert_success
    assert_output ""
    run bash -c "echo '{\"stop_hook_active\":false}' | hk agent stop-hook --target claude-code"
    assert_success
    assert_output ""
}

@test "agent stop-hook --target codex prints {} when the check passes" {
    write_stop_hook_config "true"
    run bash -c "echo '{\"stop_hook_active\":false}' | hk agent stop-hook --target codex"
    assert_success
    assert_output "{}"
}

@test "agent stop-hook blocks with only a decision when the check fails" {
    write_stop_hook_config "echo probe-failed; exit 1"
    run bash -c "echo '{}' | hk agent stop-hook 2>/dev/null"
    assert_success
    assert_output --regexp '^\{"decision":"block","reason":"[^"]*probe-failed[^"]*"\}$'
}

@test "agent stop-hook blocks when --safe refuses an unclassified command" {
    cat <<EOF > hk.pkl
amends "$PKL_PATH/Config.pkl"
hooks {
    ["check"] {
        steps {
            ["probe"] { check = "true" }
        }
    }
}
EOF
    run bash -c "echo '{}' | hk agent stop-hook 2>/dev/null"
    assert_success
    assert_output --partial '{"decision":"block","reason":"'
    assert_output --partial "--safe refused to run"
}

@test "agent stop-hook does nothing when stop_hook_active is true" {
    write_stop_hook_config "exit 1"
    run bash -c "echo '{\"stop_hook_active\":true}' | hk agent stop-hook"
    assert_success
    assert_output ""
    run bash -c "echo '{\"stop_hook_active\":true}' | hk agent stop-hook --target codex"
    assert_success
    assert_output "{}"
}

@test "agent stop-hook --timeout stops a long check and blocks once" {
    write_stop_hook_config "sleep 60"
    start=$(date +%s)
    run bash -c "echo '{}' | hk agent stop-hook --timeout 2 2>/dev/null"
    assert_success
    assert_output --partial '"decision":"block"'
    assert_output --partial "did not finish within 2 seconds"
    [ $(($(date +%s) - start)) -lt 30 ]
}

@test "agent stop-hook output stays a single decision when HK_TRACE=json is inherited" {
    write_stop_hook_config "echo probe-failed; exit 1"
    run bash -c "echo '{}' | HK_TRACE=json hk agent stop-hook 2>/dev/null"
    assert_success
    assert_output --regexp '^\{"decision":"block","reason":"[^"]*probe-failed[^"]*"\}$'
}

@test "agent stop-hook prints only {} for Codex when the check passes under HK_TRACE=json" {
    write_stop_hook_config "true"
    run bash -c "echo '{}' | HK_TRACE=json hk agent stop-hook --target codex 2>/dev/null"
    assert_success
    assert_output "{}"
}
