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
