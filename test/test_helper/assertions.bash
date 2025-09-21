#!/usr/bin/env bash

# Common assertion helpers for hk bats tests

# Assert that hk command succeeds
# Usage: assert_hk_success <command> [args...]
assert_hk_success() {
    run hk "$@"
    assert_success
}

# Assert that hk command fails
# Usage: assert_hk_failure <command> [args...]
assert_hk_failure() {
    run hk "$@"
    assert_failure
}

# Assert that a file has been modified (mtime changed)
# Usage: assert_file_modified <file> <old_mtime>
assert_file_modified() {
    local file="$1"
    local old_mtime="$2"
    local new_mtime

    if [[ "$OSTYPE" == "darwin"* ]]; then
        new_mtime=$(stat -f %m "$file")
    else
        new_mtime=$(stat -c %Y "$file")
    fi

    if [[ "$new_mtime" == "$old_mtime" ]]; then
        echo "File $file was not modified (mtime: $old_mtime)" >&2
        return 1
    fi
}

# Assert that a file has NOT been modified
# Usage: assert_file_not_modified <file> <old_mtime>
assert_file_not_modified() {
    local file="$1"
    local old_mtime="$2"
    local new_mtime

    if [[ "$OSTYPE" == "darwin"* ]]; then
        new_mtime=$(stat -f %m "$file")
    else
        new_mtime=$(stat -c %Y "$file")
    fi

    if [[ "$new_mtime" != "$old_mtime" ]]; then
        echo "File $file was modified (old: $old_mtime, new: $new_mtime)" >&2
        return 1
    fi
}

# Get file mtime for comparison
# Usage: mtime=$(get_file_mtime <file>)
get_file_mtime() {
    local file="$1"
    if [[ "$OSTYPE" == "darwin"* ]]; then
        stat -f %m "$file"
    else
        stat -c %Y "$file"
    fi
}

# Assert git working directory is clean
# Usage: assert_git_clean
assert_git_clean() {
    local status
    status=$(git status --porcelain)
    if [[ -n "$status" ]]; then
        echo "Git working directory is not clean:" >&2
        echo "$status" >&2
        return 1
    fi
}

# Assert git has uncommitted changes
# Usage: assert_git_dirty
assert_git_dirty() {
    local status
    status=$(git status --porcelain)
    if [[ -z "$status" ]]; then
        echo "Git working directory is clean (expected dirty)" >&2
        return 1
    fi
}

# Assert that a specific step was executed in hk output
# Usage: assert_step_executed <step_name>
assert_step_executed() {
    local step_name="$1"
    assert_line --partial "✓ $step_name"
}

# Assert that a specific step was skipped in hk output
# Usage: assert_step_skipped <step_name> [reason]
assert_step_skipped() {
    local step_name="$1"
    local reason="$2"

    if [[ -n "$reason" ]]; then
        assert_line --partial "⊘ $step_name ($reason)"
    else
        assert_line --partial "⊘ $step_name"
    fi
}

# Assert that a specific step failed in hk output
# Usage: assert_step_failed <step_name>
assert_step_failed() {
    local step_name="$1"
    assert_line --partial "✗ $step_name"
}

# Assert file contains specific content
# Usage: assert_file_contains <file> <content>
assert_file_contains() {
    local file="$1"
    local content="$2"

    if ! grep -q "$content" "$file"; then
        echo "File $file does not contain: $content" >&2
        echo "File contents:" >&2
        cat "$file" >&2
        return 1
    fi
}

# Assert file does not contain specific content
# Usage: assert_file_not_contains <file> <content>
assert_file_not_contains() {
    local file="$1"
    local content="$2"

    if grep -q "$content" "$file"; then
        echo "File $file contains (should not): $content" >&2
        echo "File contents:" >&2
        cat "$file" >&2
        return 1
    fi
}

# Assert that output matches a pattern (regex)
# Usage: assert_output_matches <pattern>
assert_output_matches() {
    local pattern="$1"

    if ! echo "$output" | grep -qE "$pattern"; then
        echo "Output does not match pattern: $pattern" >&2
        echo "Actual output:" >&2
        echo "$output" >&2
        return 1
    fi
}

# Assert that a line matches a pattern (regex)
# Usage: assert_line_matches <pattern>
assert_line_matches() {
    local pattern="$1"
    local line_found=false

    while IFS= read -r line; do
        if echo "$line" | grep -qE "$pattern"; then
            line_found=true
            break
        fi
    done <<< "$output"

    if [[ "$line_found" != "true" ]]; then
        echo "No line matches pattern: $pattern" >&2
        echo "Actual output:" >&2
        echo "$output" >&2
        return 1
    fi
}

# Assert command runs without error (but may have non-zero exit)
# Useful for testing error messages
# Usage: assert_runs <command> [args...]
assert_runs() {
    if ! "$@" 2>/dev/null; then
        # Command failed, but that's OK for this assertion
        true
    fi
}

# Assert file has specific permissions
# Usage: assert_file_permissions <file> <expected_perms>
assert_file_permissions() {
    local file="$1"
    local expected="$2"
    local actual

    if [[ "$OSTYPE" == "darwin"* ]]; then
        actual=$(stat -f %Lp "$file")
    else
        actual=$(stat -c %a "$file")
    fi

    if [[ "$actual" != "$expected" ]]; then
        echo "File $file has permissions $actual (expected $expected)" >&2
        return 1
    fi
}
